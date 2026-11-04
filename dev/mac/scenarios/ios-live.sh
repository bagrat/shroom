# The phone's own player against a live test-account space (worker/dev/ios-live.mjs): the
# iPhone Simulator's Mobile Safari plays the space's single-file copy and its stream beside
# the same files from loopback, and opens the space's own video page; the log, screenshots
# and verdict land in $MAC_OUT. The space is the one tests/e2e-live's a_space_held_for_a_phone
# holds up, at v-<run>.shroom-e2e.workers.dev for the run id in fixtures/phone/HOLD — start
# that (E2E (test account)) beside this. Passes when the live copy plays.
set -u
run=$(awk '{print $2}' tests/e2e-live/fixtures/phone/HOLD)
live="https://v-$run.shroom-e2e.workers.dev"
log="$MAC_OUT/ios-live.log"
echo "live: $live"
for i in $(seq 1 90); do curl -fsS -m 5 "$live/_shroom/ping" >/dev/null 2>&1 && break; sleep 10; done
curl -sS -m 5 "$live/_shroom/ping"; echo
udid=$(xcrun simctl list devices available | awk -F'[()]' '/iPhone 17 Pro \(/{print $2; exit}')
echo "simulator: $udid"
xcrun simctl boot "$udid" 2>/dev/null; open -a Simulator
xcrun simctl bootstatus "$udid" -b >/dev/null 2>&1
# Safari's first launch outlasts openurl's own wait: ask until it opens.
for i in $(seq 1 8); do xcrun simctl openurl "$udid" "about:blank" 2>/dev/null && break; sleep 15; done
tail -f /dev/null | node worker/dev/ios-live.mjs --live "$live" --mp4 PhoneMp4PhoneMp4 --hls PhoneHlsPhoneHls --log "$log" --port 8766 > "$MAC_OUT/server.out" 2>&1 &
sleep 3
for i in 1 2 3 4; do xcrun simctl openurl "$udid" "http://127.0.0.1:8766/" && break; sleep 15; done
sleep 45
xcrun simctl io "$udid" screenshot "$MAC_OUT/compare.png" >/dev/null 2>&1
xcrun simctl openurl "$udid" "$live/PhoneMp4PhoneMp4"; sleep 20
xcrun simctl io "$udid" screenshot "$MAC_OUT/page.png" >/dev/null 2>&1
echo "--- log"; cut -c1-300 "$log" | grep -v '"k":"http"'
# One line a video: how far it played by 6 s, and its error.
node -e 'const L=require("fs").readFileSync(process.argv[1],"utf8").trim().split("\n").map((l)=>{try{return JSON.parse(l)}catch{return{}}});
  let bad=0;for(const r of L.filter((x)=>x.d&&x.d.at6s)){const a=r.d.at6s;if(!(a.t>0))bad++;console.log(`${a.t>0?"plays  ":"STALLS "} ${r.k}: t=${a.t} ready=${a.ready} err=${a.err}`)}process.exit(bad?1:0)' "$log"
