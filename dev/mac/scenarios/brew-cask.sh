# Install via Homebrew, for real: the cask (installer/homebrew/shroom.rb) rendered for the
# latest public release, in a local tap, then `brew install --cask` as a person would run
# it. Passes when it lands in ~/Applications, carries no quarantine mark, is the release's
# signer, opens and, when the release carries it, has its shroom command linked; when a
# cask whose pin doesn't match installs nothing; when a copy the install line placed is
# adopted; and when `brew uninstall --zap` leaves nothing behind.
set -u
export HOMEBREW_NO_AUTO_UPDATE=1 HOMEBREW_NO_INSTALL_CLEANUP=1 HOMEBREW_NO_ENV_HINTS=1 HOMEBREW_NO_ANALYTICS=1
TAP=shroom-check/local
APP="$HOME/Applications/shroom.app"
bad=0
no() { echo "NO: $*"; bad=1; }
running() { pgrep -f "$1/Contents/MacOS/" >/dev/null 2>&1; }
quit() { osascript -e 'tell application id "am.shroom" to quit' >/dev/null 2>&1; sleep 2; pkill -f "shroom.app/Contents/MacOS/" 2>/dev/null; true; }

echo "brew $(brew --version | head -n 1)"
brew tap-new --no-git "$TAP" >/dev/null || { echo "NO: couldn't make a local tap"; exit 1; }
casks="$(brew --repository)/Library/Taps/shroom-check/homebrew-local/Casks"
mkdir -p "$casks"
installer/homebrew/render-cask.sh >"$casks/shroom.rb" || { echo "NO: couldn't render the cask"; exit 1; }
grep -E '^  (version|sha256) ' "$casks/shroom.rb"
rm -rf "$APP"

echo "── brew style, brew audit (for the record)"
brew style --cask "$TAP/shroom" 2>&1 | tail -n 30
brew audit --cask --online "$TAP/shroom" 2>&1 | tail -n 10
brew livecheck --cask "$TAP/shroom" 2>&1 | tail -n 3

echo "── install"
brew install --cask "$TAP/shroom"; rc=$?
[ "$rc" = 0 ] || no "brew install exited $rc"
[ -d "$APP" ] && echo "placed: $APP" || no "nothing at $APP"
[ -e /Applications/shroom.app ] && no "a copy in /Applications too"
q=$(xattr -lr "$APP" 2>/dev/null | grep -c com.apple.quarantine)
[ "$q" = 0 ] && echo "quarantine: none" || no "$q files still carry the quarantine mark"
leaf=$(tr -d ' \n' <tools/release-identity.sha1)
codesign --verify --deep --strict -R "=identifier \"am.shroom\" and certificate leaf = H\"$leaf\"" "$APP" \
  && echo "signer: the release's" || no "not the release's signer"
brew info --cask "$TAP/shroom" 2>&1 | grep -v "^::warning" | sed -n '1,6p'
open "$APP"; sleep 8
running "$APP" && echo "opened: it runs" || no "it didn't open"
quit

echo "── the shroom command (spec N62, DECISIONS B217)"
# A release whose manifest says it carries the command gets the cask's `binary`: Homebrew
# links `shroom` to the app's own, and it answers with the release's version.
CMD="$(brew --prefix)/bin/shroom"
carries=$(curl -fsSL https://github.com/bagrat/shroom/releases/latest/download/manifest.txt | sed -n 's/^cli=//p')
version=$(sed -n 's/^  version "\(.*\)"$/\1/p' "$casks/shroom.rb")
if [ "$carries" = shroom-cli ]; then
  grep -E '^  binary ' "$casks/shroom.rb" || no "the cask has no binary for a release that carries the command"
  to=$(readlink "$CMD" 2>/dev/null)
  [ "$to" = "$APP/Contents/MacOS/shroom-cli" ] && echo "command: $CMD → $to" || no "Homebrew linked no command to the app's own (${to:-nothing at $CMD})"
  said=$("$CMD" --version 2>&1)
  echo "command: $said"
  case $said in
    "shroom $version") ;;
    # 0.1.25's command named its crate's version; #73 fixed it from 0.1.26.
    "shroom 0.1.0") [ "$version" = 0.1.25 ] && echo "  (0.1.25's known slip: fixed from 0.1.26)" || no "the command says $said, not shroom $version" ;;
    *) no "the command says ${said:-nothing}, not shroom $version" ;;
  esac
else
  echo "the release doesn't carry the command: no binary (cask: $(grep -cE '^  binary ' "$casks/shroom.rb"))"
fi

echo "── control (for the record): the same app with the mark Homebrew adds, opened"
ctl=$(mktemp -d)/shroom.app
ditto "$APP" "$ctl"
xattr -w com.apple.quarantine "0081;$(printf %x "$(date +%s)");Homebrew Cask;" "$ctl"
spctl --assess --type execute -vv "$ctl" 2>&1 | sed 's/^/  spctl: /'
open "$ctl" >/dev/null 2>&1 &
sleep 10
running "$ctl" && echo "  marked copy: it runs (the mark wouldn't have stopped it)" || echo "  marked copy: macOS didn't let it run — why the cask lifts the mark"
kill $! 2>/dev/null
quit
pkill -x CoreServicesUIAgent 2>/dev/null
rm -rf "$(dirname "$ctl")"

echo "── uninstall --zap"
mkdir -p "$HOME/.shroom" "$HOME/Library/Caches/am.shroom"; : >"$HOME/.shroom/marker"
brew uninstall --zap --cask "$TAP/shroom"; rc=$?
[ "$rc" = 0 ] || no "brew uninstall exited $rc"
for p in "$APP" "$HOME/.shroom" "$HOME/Library/Caches/am.shroom"; do [ -e "$p" ] && no "left: $p"; done
{ [ -e "$CMD" ] || [ -L "$CMD" ]; } && no "left: $CMD"
[ "$bad" = 0 ] && echo "zap: nothing left"

echo "── a cask whose pin isn't the signer's installs nothing"
sed -i '' 's/^      leaf=[0-9a-f]*$/      leaf=0000000000000000000000000000000000000000/' "$casks/shroom.rb"
grep -q '^      leaf=0000' "$casks/shroom.rb" || no "couldn't write the wrong pin"
said=$(brew install --cask "$TAP/shroom" 2>&1); rc=$?
printf '%s\n' "$said" | tail -n 4 | sed 's/^/  /'
[ "$rc" != 0 ] && echo "refused: exit $rc" || no "installed with the wrong pin"
[ -e "$APP" ] && no "an app was left at $APP"
brew list --cask 2>/dev/null | grep -qx shroom && no "Homebrew lists it as installed"
brew uninstall --cask --force "$TAP/shroom" >/dev/null 2>&1
installer/homebrew/render-cask.sh >"$casks/shroom.rb"

echo "── adopt the install line's copy"
sh installer/install.sh; rc=$?
[ "$rc" = 0 ] || no "the install line exited $rc"
quit
brew install --cask --adopt "$TAP/shroom"; rc=$?
[ "$rc" = 0 ] && echo "adopted" || no "brew install --adopt exited $rc"
q=$(xattr -lr "$APP" 2>/dev/null | grep -c com.apple.quarantine)
[ "$q" = 0 ] || no "$q files carry the quarantine mark after adopting"
brew uninstall --zap --cask "$TAP/shroom" >/dev/null 2>&1
[ -e "$APP" ] && no "left after adopting and uninstalling: $APP"

brew untap "$TAP" >/dev/null 2>&1
[ "$bad" = 0 ] && echo "verdict: Homebrew installs, opens, refuses a wrong signer, adopts and zaps"
exit "$bad"
