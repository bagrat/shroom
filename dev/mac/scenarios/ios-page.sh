# The page in an iPhone's own Safari: the iPhone Simulator's Mobile Safari opens the real
# worker.js, served over loopback by worker/dev/ios-page.mjs from a take encoded the way the
# app encodes (VideoToolbox H.264, AAC, fMP4 HLS), and Play, the picture and fullscreen are
# tapped through Simulator. What the page did (taps, clicks, the video's events, errors) is
# in page.log; a screenshot after each step. Passes when a tap on Play plays.
# mac-limit: 900
set -u
cd "$(dirname "$0")/../../.."
out=$MAC_OUT
media="$out/media"
mkdir -p "$media/v0"

ffmpeg=$(find "$MAC_APP" -type f -name ffmpeg 2>/dev/null | head -n 1)
[ -n "$ffmpeg" ] || ffmpeg=$(command -v ffmpeg || ls /opt/homebrew/bin/ffmpeg /usr/local/bin/ffmpeg 2>/dev/null | head -n 1)
[ -n "$ffmpeg" ] || { echo "NO: no ffmpeg to encode a take"; exit 1; }
echo "ffmpeg: $ffmpeg"
venc=libx264
"$ffmpeg" -hide_banner -encoders 2>/dev/null | grep -q h264_videotoolbox && venc=h264_videotoolbox
echo "video encoder: $venc"
(cd "$media/v0" && "$ffmpeg" -hide_banner -loglevel error -f lavfi -i testsrc2=size=1280x800:rate=30 -f lavfi -i sine=frequency=440 -t 20 \
  -c:v "$venc" -b:v 3M -pix_fmt yuv420p -g 60 -c:a aac -f hls -hls_time 2 -hls_playlist_type vod -hls_segment_type fmp4 \
  -hls_fmp4_init_filename init.mp4 -hls_segment_filename 'seg_%05d.m4s' stream.m3u8) || { echo "NO: the encode failed"; exit 1; }
"$ffmpeg" -hide_banner -loglevel error -f lavfi -i testsrc2=size=1280x800 -frames:v 1 "$media/poster.jpg"
printf '#EXTM3U\n#EXT-X-VERSION:7\n#EXT-X-STREAM-INF:BANDWIDTH=3500000,RESOLUTION=1280x800\nv0/stream.m3u8\n' >"$media/master.m3u8"
"$ffmpeg" -hide_banner -loglevel error -i "$media/v0/stream.m3u8" -c copy -movflags +faststart "$media/video.mp4"
echo 20 >"$media/duration.txt"

# The server, until this script ends.
mkfifo "$out/stdin"
node worker/dev/ios-page.mjs --media "$media" --log "$out/page.log" --port 8799 <"$out/stdin" >"$out/server.json" &
exec 9>"$out/stdin"
for _ in $(seq 1 40); do [ -s "$out/server.json" ] && break; sleep 0.5; done
cat "$out/server.json"

# An iPhone.
udid=$(xcrun simctl list devices available -j | node -e 'const d=JSON.parse(require("fs").readFileSync(0,"utf8")).devices;const all=Object.entries(d).filter(([r])=>/iOS/.test(r)).flatMap(([r,l])=>l.map(x=>({...x,r})));const p=all.filter(x=>/^iPhone \d+$/.test(x.name)).pop()||all.find(x=>/iPhone/.test(x.name));console.log(p?p.udid:"")')
[ -n "$udid" ] || { echo "NO: no iPhone simulator here"; xcrun simctl list runtimes; exit 1; }
xcrun simctl list devices | grep "$udid"
xcrun simctl boot "$udid" 2>/dev/null
open -a Simulator --args -CurrentDeviceUDID "$udid"
xcrun simctl bootstatus "$udid" -b >/dev/null 2>&1
sleep 5
echo "$udid" >"$out/udid"

shot() { xcrun simctl io "$udid" screenshot "$out/$1.png" >/dev/null 2>&1; echo "shot: $1.png"; }
# Taps the element Simulator shows to macOS accessibility with this label (the page's
# aria-label), at its centre, as a finger would.
tap() {
  local at
  at=$(osascript -l JavaScript dev/mac/ax.js Simulator 2>/dev/null | awk -F'\t' -v l="$1" '$3==l {print $4; exit}')
  echo "tap $1: ${at:-not found}"
  [ -n "$at" ] && cliclick "c:$at"
}
open_page() { xcrun simctl openurl "$udid" "http://127.0.0.1:8799/$1"; sleep 8; }

open_page "StrmStrmStrmStrm"
shot stream-0
tap "Play"; sleep 5; shot stream-1-play
tap "Fullscreen"; sleep 4; shot stream-2-fullscreen
osascript -l JavaScript dev/mac/ax.js Simulator >"$out/ax-dump.txt" 2>&1

open_page "NamdNamdNamdNamd"
shot named-0
tap "Play"; sleep 5; shot named-1-play

open_page "OneFOneFOneFOneF?probe"
sleep 6; shot single-probe

exec 9>&-
sleep 1
grep -q '"t":"playing"' "$out/page.log" && grep -q '"page":"/StrmStrmStrmStrm".*"t":"click"' "$out/page.log" && { echo "OK: a tap on Play played"; exit 0; }
echo "NO: a tap on Play didn't play (page.log)"; exit 1
