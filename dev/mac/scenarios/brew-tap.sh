# Install via Homebrew from the public tap, as a person would: `brew install --cask
# bagrat/shroom/shroom` on a Mac that never tapped it. Passes when it lands in
# ~/Applications with no quarantine mark, is the release's signer, opens, and
# `brew uninstall --zap` leaves nothing; and an app that carries the shroom command has it
# linked. (brew-cask.sh tries the cask on this branch.)
set -u
export HOMEBREW_NO_AUTO_UPDATE=1 HOMEBREW_NO_INSTALL_CLEANUP=1 HOMEBREW_NO_ENV_HINTS=1 HOMEBREW_NO_ANALYTICS=1
APP="$HOME/Applications/shroom.app"
bad=0
no() { echo "NO: $*"; bad=1; }
rm -rf "$APP"
brew install --cask bagrat/shroom/shroom; rc=$?
[ "$rc" = 0 ] || no "brew install exited $rc"
q=$(xattr -lr "$APP" 2>/dev/null | grep -c com.apple.quarantine)
[ -d "$APP" ] && [ "$q" = 0 ] && echo "placed, no quarantine mark" || no "not placed, or $q files marked"
leaf=$(tr -d ' \n' <tools/release-identity.sha1)
codesign --verify --deep --strict -R "=identifier \"am.shroom\" and certificate leaf = H\"$leaf\"" "$APP" \
  && echo "signer: the release's" || no "not the release's signer"
open "$APP"; sleep 8
pgrep -f "$APP/Contents/MacOS/" >/dev/null && echo "opened: it runs" || no "it didn't open"
# The shroom command (spec N62, DECISIONS B217): an app that carries it gets it linked by
# the cask's `binary`, to the app's own.
CMD="$(brew --prefix)/bin/shroom"
if [ -x "$APP/Contents/MacOS/shroom-cli" ]; then
  to=$(readlink "$CMD" 2>/dev/null)
  [ "$to" = "$APP/Contents/MacOS/shroom-cli" ] && echo "command: $CMD → $to, says $("$CMD" --version 2>&1)" || no "the app carries the command and the tap linked none (${to:-nothing at $CMD})"
else
  echo "command: this release carries none ($(brew info --cask bagrat/shroom/shroom 2>/dev/null | head -n 1))"
fi
osascript -e 'tell application id "am.shroom" to quit' >/dev/null 2>&1; sleep 2
brew uninstall --zap --cask bagrat/shroom/shroom || no "brew uninstall exited $?"
[ -e "$APP" ] && no "left: $APP"
{ [ -e "$CMD" ] || [ -L "$CMD" ]; } && no "left: $CMD"
brew untap bagrat/shroom >/dev/null 2>&1
[ "$bad" = 0 ] && echo "verdict: the public tap installs, opens and zaps"
exit "$bad"
