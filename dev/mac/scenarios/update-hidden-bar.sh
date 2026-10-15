# The update drive with the menu bar set to hide (DECISIONS B161): the popover an update
# relaunch opens by itself finds the mushroom parked over the hidden bar, and must still
# hang under it ("it hangs under the mushroom"). The pointer waits mid-screen, where it
# can't bring the bar back; the app's own popover lines are kept (popover.log). The bar is
# shown again either way.
set -u
hide() {
  defaults write NSGlobalDomain _HIHideMenuBar -bool "$1"
  osascript -l JavaScript -e 'ObjC.import("Foundation"); $.NSDistributedNotificationCenter.defaultCenter.postNotificationNameObjectUserInfoDeliverImmediately("AppleInterfaceMenuBarHidingChangedNotification", $(), $(), true)' >/dev/null
}
keep() {
  while :; do
    find "${TMPDIR:-/tmp}" -path '*shroom-r5-update-*' -name 'run.log' -exec grep -h 'popover: ' {} + 2>/dev/null
    sleep 0.5
  done | awk '!seen[$0]++ { print; fflush() }' >"$MAC_OUT/popover.log"
}
keep &
watcher=$!
trap 'kill $watcher 2>/dev/null; pkill -P $watcher 2>/dev/null; defaults write NSGlobalDomain _HIHideMenuBar -bool false; hide false; defaults delete NSGlobalDomain _HIHideMenuBar 2>/dev/null; echo "the app said:"; cat "$MAC_OUT/popover.log"' EXIT
hide true
osascript -l JavaScript -e 'ObjC.import("CoreGraphics"); $.CGWarpMouseCursorPosition({ x: 512, y: 500 })' >/dev/null
sleep 2
sh app/test/update-drive.sh
