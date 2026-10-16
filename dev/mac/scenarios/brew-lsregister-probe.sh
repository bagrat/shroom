# A probe, not a check: can the cask tell Launch Services about shroom as it installs, so
# `open -a shroom` and `shroom://` work the moment `brew install` returns? (brew-join-probe
# found neither does by itself.) Homebrew 7 runs a cask's steps in a sandbox whose $HOME is
# a temporary folder (DECISIONS B175), so each way is tried on the real release through a
# local tap: a postflight_steps run step, a preflight_steps one on the staged copy, and the
# deprecated Ruby postflight block. Reports what happened; the verdict is read.
set -u
export HOMEBREW_NO_AUTO_UPDATE=1 HOMEBREW_NO_INSTALL_CLEANUP=1 HOMEBREW_NO_ENV_HINTS=1 HOMEBREW_NO_ANALYTICS=1
TAP=shroom-check/local
APP="$HOME/Applications/shroom.app"
OUT=${MAC_OUT:-$(mktemp -d)}
LSREG=/System/Library/Frameworks/CoreServices.framework/Frameworks/LaunchServices.framework/Support/lsregister
SLINK="shroom://join/probe-owner.example.workers.dev/AbCdEfGh12345678"

cat >"$OUT/ls.swift" <<'EOF'
import AppKit
for a in CommandLine.arguments.dropFirst() {
  if a.contains(":") { let u = URL(string: a)!; print("  scheme \(u.scheme!): \(NSWorkspace.shared.urlForApplication(toOpen: u)?.path ?? "none")") }
  else { print("  name \(a): \(NSWorkspace.shared.fullPath(forApplication: a) ?? "none")") }
}
EOF
swiftc -O -o "$OUT/ls" "$OUT/ls.swift" 2>&1 | grep -v -i warning | tail -n 3

echo "brew $(brew --version 2>/dev/null | head -n 1)"
echo "── what Homebrew 7's steps offer (its own source)"
src="$(brew --repository)/Library/Homebrew"
grep -rhoE '\{\{[a-z_]+\}\}' "$src/cask" 2>/dev/null | sort | uniq -c | sed 's/^/  /'
grep -rlE 'postflight_steps|preflight_steps' "$src/cask" 2>/dev/null | sed 's/^/  file: /' | head -n 8

brew tap-new --no-git "$TAP" >/dev/null || { echo "couldn't make a local tap"; exit 1; }
casks="$(brew --repository)/Library/Taps/shroom-check/homebrew-local/Casks"
mkdir -p "$casks"
installer/homebrew/render-cask.sh >"$OUT/base.rb" || { echo "couldn't render the cask"; exit 1; }

# The step's shell: what it sees, then lsregister on the placed copy found by the person's
# real home (from the directory service, not $HOME).
STEP='
      echo "step: HOME=$HOME user=$(id -un) arg=$1" >&2
      real=$(/usr/bin/dscl . -read "/Users/$(id -un)" NFSHomeDirectory 2>/dev/null | sed "s/^NFSHomeDirectory: //")
      echo "step: real home=$real; placed copy there: $([ -d "$real/Applications/shroom.app" ] && echo yes || echo no); arg exists: $([ -d "$1" ] && echo yes || echo no)" >&2
      /System/Library/Frameworks/CoreServices.framework/Frameworks/LaunchServices.framework/Support/lsregister -f "${1:-$real/Applications/shroom.app}" >&2; echo "step: lsregister exit $?" >&2
      true'

variant() { # <name> <ruby to insert before `uninstall quit:`>
  local name=$1
  INS="$2" awk '/^  uninstall quit:/ { print ENVIRON["INS"]; print "" } { print }' "$OUT/base.rb" >"$casks/shroom.rb"
  echo "══ $name"
  sed -n '/^  postflight/,/^  end$/p' "$casks/shroom.rb" | head -n 3 | sed 's/^/  cask: /'
  brew uninstall --zap --cask "$TAP/shroom" >/dev/null 2>&1
  "$LSREG" -u "$APP" >/dev/null 2>&1
  rm -rf "$APP"
  "$OUT/ls" shroom "$SLINK"
  brew install --cask "$TAP/shroom" >"$OUT/$name.log" 2>&1; rc=$?
  echo "  brew exit $rc"
  grep -iE 'step:|warning|deprecat|error|sandbox' "$OUT/$name.log" | sed 's/^/  brew: /' | head -n 12
  "$OUT/ls" shroom "$SLINK"
  said=$(open -a shroom 2>&1); echo "  open -a shroom: exit $? ${said:+said: $said}"
  sleep 4
  pgrep -f "$APP/Contents/MacOS/" >/dev/null && echo "  shroom: running" || echo "  shroom: NOT running"
  osascript -e 'tell application id "am.shroom" to quit' >/dev/null 2>&1; sleep 2; pkill -f "shroom.app/Contents/MacOS/" 2>/dev/null
}

variant control ""

variant postflight-steps-home "  postflight_steps do
    run \"/bin/sh\", args: [\"-c\", <<~'SH', \"shroom-register\"]$STEP
    SH
  end"

variant postflight-steps-appdir "  postflight_steps do
    run \"/bin/sh\", args: [\"-c\", <<~'SH', \"shroom-register\", \"{{appdir}}/shroom.app\"]$STEP
    SH
  end"

variant preflight-staged "  preflight_steps do
    run \"/bin/sh\", args: [\"-c\", <<~'SH', \"shroom-register\", \"{{staged_path}}/shroom.app\"]$STEP
    SH
  end"

variant ruby-postflight "  postflight do
    system_command \"$LSREG\", args: [\"-f\", \"#{Dir.home}/Applications/shroom.app\"]
  end"

echo "══ the last state, and a plain open of the link"
"$OUT/ls" shroom "$SLINK"
brew uninstall --zap --cask "$TAP/shroom" >/dev/null 2>&1
brew untap "$TAP" >/dev/null 2>&1
echo "done — read, not asserted"
exit 0
