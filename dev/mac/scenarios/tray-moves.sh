# Where macOS keeps a status item while it changes face, while the menu bar hides, and
# while a full-screen app has the screen (DECISIONS B161's probe): a throwaway status item
# made by tray-moves.js, read every 10 ms. Prints each change; info only.
set -u
. dev/mac/lib.sh
limit 90 osascript -l JavaScript dev/mac/probes/tray-moves.js 2>&1 | tee "$MAC_OUT/tray-moves.log"
screencapture -x "$MAC_OUT/screen.png"
