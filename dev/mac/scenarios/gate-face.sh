# The card gate's face, held: the onboarding drive's card-gate path with the gate left up
# for 40 s, and the screen shot every 8 s meanwhile — whether the window draws the gate's
# line and Add credit card, or stays on the card step's spinner (CHECKPOINT §10, 3.2).
( for i in 1 2 3 4 5 6 7 8 9 10; do sleep 8; screencapture -x "$MAC_OUT/gate-$i.png"; done ) &
shots=$!
GATE_HOLD_MS=40000 node app/test/onboarding-drive.mjs card-gate
code=$?
kill "$shots" 2>/dev/null
exit $code
