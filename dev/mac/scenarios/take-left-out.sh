# A take from a cold start, and what it leaves out of its picture: the popover and the count
# are shown over the test pattern while the take runs, and every frame of it, decoded here,
# must be the pattern alone.
SHROOM_TAKE_OUT="$MAC_OUT/left-out" node app/test/take-drive.mjs left-out &&
  node tests/e2e/left-out.mjs "$MAC_OUT/left-out/take"
