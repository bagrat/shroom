# The popover, the count and the bubble as macOS has them: who has the keyboard, who is in
# front, which Space they sit on, whether a first click acts — beside another app that goes
# full screen. Nothing is recorded. A hosted runner has no camera; the drive names what it
# skipped, and anything else skipped fails it.
node app/test/surfaces-drive.mjs --fullscreen --strict --report="$MAC_OUT/surfaces.json"
