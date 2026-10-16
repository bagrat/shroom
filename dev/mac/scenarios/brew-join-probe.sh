# A probe, not a check: what a one-line Homebrew install-and-join would do on a fresh Mac.
# Installs the public tap's cask as a person would, then hands shroom a join link by
# `open -a`, and does the same with two throwaway test apps (no URL types; one custom
# scheme) to tell macOS's behaviour apart from shroom's. Reports what happened; the
# verdict is read, not asserted. The link is invented and reaches nothing real.
#
# Run 1 (38875304073) found `open -a shroom` right after the install says "Unable to find
# application named 'shroom'", and the scheme unbound until `lsregister -f`. This run asks
# why, how long until Launch Services learns of it, and what opening by path delivers.
set -u
export HOMEBREW_NO_AUTO_UPDATE=1 HOMEBREW_NO_INSTALL_CLEANUP=1 HOMEBREW_NO_ENV_HINTS=1 HOMEBREW_NO_ANALYTICS=1
APP="$HOME/Applications/shroom.app"
LINK="https://probe-owner.example.workers.dev/join/AbCdEfGh12345678"
SLINK="shroom://join/probe-owner.example.workers.dev/AbCdEfGh12345678"
OUT=${MAC_OUT:-$(mktemp -d)}
LSREG=/System/Library/Frameworks/CoreServices.framework/Frameworks/LaunchServices.framework/Support/lsregister
shot() { screencapture -x "$OUT/$1.png" 2>/dev/null; echo "  screenshot: $1.png"; }
quit() { osascript -e 'tell application id "am.shroom" to quit' >/dev/null 2>&1; sleep 2; pkill -f "shroom.app/Contents/MacOS/" 2>/dev/null; sleep 1; true; }
running() { pgrep -f "$APP/Contents/MacOS/" >/dev/null && echo "  shroom: running" || echo "  shroom: NOT running"; }
browsers() { pgrep -lx Safari 2>/dev/null | sed 's/^/  browser running: /'; true; }
args_of() { ps -axo args | grep "$1/Contents/MacOS/" | grep -v grep | sed 's/^/  argv: /'; }
logtail() { [ -f "$HOME/.shroom/logs/run.log" ] && grep -E "member:|invite|link|launch|Booted" "$HOME/.shroom/logs/run.log" | sed 's/^/  log: /' | tail -n 8; ls -l "$HOME/.shroom/invite" 2>/dev/null | sed 's/^/  file: /'; true; }

# What Launch Services knows: an app by name (what `open -a <name>` asks), by id, and
# which app a URL's scheme goes to with plain `open`.
cat >"$OUT/ls.swift" <<'EOF'
import AppKit
for a in CommandLine.arguments.dropFirst() {
  if a.contains(":") { let u = URL(string: a)!; print("  scheme \(u.scheme!): \(NSWorkspace.shared.urlForApplication(toOpen: u)?.path ?? "none")") }
  else if a.contains(".") { print("  id \(a): \(NSWorkspace.shared.urlForApplication(withBundleIdentifier: a)?.path ?? "none")") }
  else { print("  name \(a): \(NSWorkspace.shared.fullPath(forApplication: a) ?? "none")") }
}
EOF
swiftc -O -o "$OUT/ls" "$OUT/ls.swift" 2>&1 | grep -v warning | tail -n 3
# Polls until the name resolves (or <secs> pass); prints how long.
wait_name() { # <name> <secs>
  local i=0
  while [ $i -lt "$2" ]; do "$OUT/ls" "$1" | grep -qv ': none$' && { echo "  name $1 resolved after ${i}s"; return 0; }; sleep 1; i=$((i + 1)); done
  echo "  name $1 still unknown after ${2}s"; return 1
}

echo "macOS $(sw_vers -productVersion)"
echo "── the runner"
mdutil -s / 2>&1 | sed 's/^/  spotlight: /'
[ -d "$HOME/Applications" ] && echo "  ~/Applications exists before install" || echo "  ~/Applications does NOT exist before install"
pgrep -lx lsd | sed 's/^/  /'; pgrep -lx mds | sed 's/^/  /'
"$OUT/ls" shroom am.shroom "$LINK" "$SLINK"

echo "══ A. brew install, then wait to see whether Launch Services learns of shroom by itself"
rm -rf "$APP"
brew install --cask bagrat/shroom/shroom 2>&1 | tail -n 2
"$OUT/ls" shroom am.shroom "$SLINK"
wait_name shroom 60
"$OUT/ls" am.shroom "$SLINK"
echo "── open -b am.shroom (by id) now?"
said=$(open -b am.shroom --hide 2>&1); echo "  exit $? ${said:+said: $said}"; sleep 3; running; quit

echo "══ B. the https link, by path: open -a ~/Applications/shroom.app \"<https link>\" (cold)"
said=$(open -a "$APP" "$LINK" 2>&1); echo "  exit $? ${said:+said: $said}"
sleep 12; running; args_of "$APP"; browsers; logtail; shot b-https-cold
echo "── after it has run once, does Launch Services know it?"
"$OUT/ls" shroom am.shroom "$SLINK"
echo "── the same link warm"
open -a "$APP" "$LINK"; echo "  exit $?"; sleep 5; logtail; shot b-https-warm
quit

echo "══ C. the shroom:// link, by path, cold"
rm -f "$HOME/.shroom/invite"
said=$(open -a "$APP" "$SLINK" 2>&1); echo "  exit $? ${said:+said: $said}"
sleep 12; running; args_of "$APP"; logtail; shot c-scheme-cold
quit
echo "── plain open \"<shroom:// link>\" (after one launch)"
rm -f "$HOME/.shroom/invite"
said=$(open "$SLINK" 2>&1); echo "  exit $? ${said:+said: $said}"
sleep 10; running; logtail; shot c-scheme-plain
quit

echo "══ D. the README line on a fresh install: brew install && open -a shroom, and the path form"
brew uninstall --zap --cask bagrat/shroom/shroom >/dev/null 2>&1
"$OUT/ls" shroom am.shroom
brew install --cask bagrat/shroom/shroom >/dev/null 2>&1 && open -a shroom; echo "  name form: exit $?"
sleep 3; running; quit
brew uninstall --zap --cask bagrat/shroom/shroom >/dev/null 2>&1
brew install --cask bagrat/shroom/shroom >/dev/null 2>&1 && open ~/Applications/shroom.app; echo "  path form: exit $?"
sleep 8; running; shot d-path-form; quit

echo "══ E. the test apps (what macOS itself does), moved into the existing ~/Applications"
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
  swiftc -O -o "$b/Contents/MacOS/probe" "$OUT/probe.swift" 2>&1 | grep -v warning | tail -n 3
  types=""
  [ "$kind" = scheme ] && types='<key>CFBundleURLTypes</key><array><dict><key>CFBundleURLName</key><string>am.shroom.probe</string><key>CFBundleURLSchemes</key><array><string>shroomprobe</string></array></dict></array>'
  cat >"$b/Contents/Info.plist" <<EOF
<?xml version="1.0" encoding="UTF-8"?><!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict><key>CFBundleIdentifier</key><string>am.shroom.probe.$kind</string><key>CFBundleName</key><string>probe-$kind</string>
<key>CFBundleExecutable</key><string>probe</string><key>CFBundlePackageType</key><string>APPL</string><key>LSUIElement</key><true/>$types</dict></plist>
EOF
  codesign -s - --force "$b" >/dev/null 2>&1
  rm -f "/tmp/am.shroom.probe.$kind.log"
  mv "$b" "$HOME/Applications/"
done
"$OUT/ls" probe-plain probe-scheme "shroomprobe://x"
wait_name probe-scheme 30
"$OUT/ls" "shroomprobe://x"

echo "── E1. open -a <path>/probe-plain.app \"<https link>\", cold, no URL types"
said=$(open -a "$HOME/Applications/probe-plain.app" "$LINK" 2>&1); echo "  exit $? ${said:+said: $said}"
sleep 4; sed 's/^/  app: /' /tmp/am.shroom.probe.plain.log 2>/dev/null || echo "  app: no log"; browsers
echo "── E1 warm"
open -a "$HOME/Applications/probe-plain.app" "$LINK"; sleep 2; tail -n 1 /tmp/am.shroom.probe.plain.log | sed 's/^/  app: /'
echo "── plain open of the https link still goes to:"; "$OUT/ls" "$LINK"
pkill -x probe
echo "── E2. open -a <path>/probe-scheme.app \"shroomprobe://…\", cold"
said=$(open -a "$HOME/Applications/probe-scheme.app" "shroomprobe://join/with-a" 2>&1); echo "  exit $? ${said:+said: $said}"
sleep 4; sed 's/^/  app: /' /tmp/am.shroom.probe.scheme.log 2>/dev/null || echo "  app: no log"
"$OUT/ls" probe-scheme "shroomprobe://x"
pkill -x probe

echo "══ F. with Spotlight on, as on a person's Mac: a fresh test app moved in"
sudo mdutil -i on / 2>&1 | sed 's/^/  /'; sleep 5; mdutil -s / 2>&1 | sed 's/^/  spotlight: /'
b="$stage/probe-late.app"; mkdir -p "$b/Contents/MacOS"
cp "$HOME/Applications/probe-scheme.app/Contents/MacOS/probe" "$b/Contents/MacOS/"
sed 's/probe\.scheme/probe.late/; s/probe-scheme/probe-late/; s/<string>shroomprobe</<string>shroomlate</' "$HOME/Applications/probe-scheme.app/Contents/Info.plist" >"$b/Contents/Info.plist"
codesign -s - --force "$b" >/dev/null 2>&1
mv "$b" "$HOME/Applications/"
wait_name probe-late 60
"$OUT/ls" "shroomlate://x"
echo "── and shroom installed again with Spotlight on"
brew uninstall --zap --cask bagrat/shroom/shroom >/dev/null 2>&1
"$LSREG" -u "$APP" >/dev/null 2>&1
"$OUT/ls" shroom
brew install --cask bagrat/shroom/shroom >/dev/null 2>&1 && open -a shroom; echo "  name form: exit $?"
wait_name shroom 60
sleep 2; running; quit

echo "══ G. lsregister -f on the installed app, then name and scheme"
"$LSREG" -f "$APP"; echo "  lsregister exit $?"
"$OUT/ls" shroom "$SLINK"

rm -rf "$HOME/Applications/probe-plain.app" "$HOME/Applications/probe-scheme.app" "$HOME/Applications/probe-late.app"
brew uninstall --zap --cask bagrat/shroom/shroom >/dev/null 2>&1
echo "done — read, not asserted"
exit 0
