# What one view of the single-file copy costs in real Safari (DECISIONS B176): the real
# worker.js over loopback, a 3-minute 1080p recording at Normal's bitrate, and Safari driven
# through safaridriver. A page opened and never played, a full watch and a watch with two
# seeks, at real speed and muted (at 8× Safari asks for far more: run 38877481672). The
# stream is left out to stay inside the scenario's 10 minutes; at real speed its full watch
# of a 5-minute video was 58 (run 38878595183). Passes when every view plays through and
# asks for no piece of the stream; the counts are the evidence (view-cost.json).
set -u
cd worker || exit 1
sudo safaridriver --enable || { echo "NO: couldn't turn on Safari's remote automation"; exit 1; }
node dev/view-cost.mjs --engines safari --secs 180 --seg 6 --rate 1 --mp4 --only mp4 >"$MAC_OUT/view-cost.json" 2>"$MAC_OUT/view-cost.log"
rc=$?
cat "$MAC_OUT/view-cost.log"
[ "$rc" = 0 ] && echo "verdict: Safari plays the copy and the stream through; the counts are above" || echo "NO: a view didn't play through, or the copy's view asked for the stream (exit $rc)"
exit "$rc"
