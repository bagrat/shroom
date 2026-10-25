# A take from a cold start, and what it leaves out of its picture: the popover, the count and
# the agent label are shown over the test pattern while the take runs, ⌥⌘M's marker toast
# comes up from the shortcut's own event (N78), and every frame of it, decoded here, must
# be the pattern alone.
SHROOM_TAKE_OUT="$MAC_OUT/left-out" node app/test/take-drive.mjs left-out &&
  node tests/e2e/left-out.mjs "$MAC_OUT/left-out/take"
