# What one view costs in real Safari (DECISIONS B176): the real worker.js over loopback, a
# 5-minute 1080p recording at Normal's bitrate, and Safari driven through safaridriver —
# native HLS for the stream, the file itself for the single-file copy. Per source: a page
# opened and never played, a full watch and a watch with two seeks (real speed, muted: Safari asks differently at 8×, run 38877481672). Passes
# when every view plays through and the copy's views ask for no piece of the stream; the
# counts are the evidence (view-cost.json).
set -u
cd worker || exit 1
sudo safaridriver --enable || { echo "NO: couldn't turn on Safari's remote automation"; exit 1; }
node dev/view-cost.mjs --engines safari --secs 300 --seg 6 --rate 1 --mp4 >"$MAC_OUT/view-cost.json" 2>"$MAC_OUT/view-cost.log"
rc=$?
cat "$MAC_OUT/view-cost.log"
[ "$rc" = 0 ] && echo "verdict: Safari plays the copy and the stream through; the counts are above" || echo "NO: a view didn't play through, or the copy's view asked for the stream (exit $rc)"
exit "$rc"
