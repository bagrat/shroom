# The pre-merge e2e run (TESTING-PLAN D3; .github/workflows/e2e-full.yml), on the dedicated
# test account: the installed app's own wizard with the run's token, then the heartbeat in
# the space it wired — one take, the page in WebKit and Chromium, every frame and the sound,
# the owner's title, a member invited, joining, recording and removed, delete. Only that
# workflow runs it: it needs the run opened (E2E_TOKEN_FILE) and the account's id in place.
#
# Exit 2 from the heartbeat is the whole path holding with a timing bar missed (repeated or
# skipped frames, the A/V offset): a hosted runner is three virtual cores under its own
# build's load, so it never decides those bars — ci.yml's rule for its record tests. The
# scenario passes and leaves the missed bars in bars.txt, which the workflow raises as a
# warning on the run and the status; a Mac decides them (heartbeat.mjs, never loosened).
SHROOM_INSTALLED="$MAC_APP" node tests/e2e/full.mjs 10
code=$?
# Voice cuts end to end (N65, tests/e2e/voice-cut.mjs), when the request asks for them (a
# line `voice-cut` in dev/e2e/request): after the heartbeat, in the run's own space, on a build
# that turns the AI on (N66). A voice-cut run that didn't hold fails the scenario.
if grep -qx 'voice-cut' dev/e2e/request 2>/dev/null; then
  SHROOM_INSTALLED="$MAC_APP" node tests/e2e/voice-cut.mjs || code=1
fi
# What the browsers saw, as MP4s a phone plays (Playwright keeps WebM).
for f in "$MAC_OUT"/page/*.webm; do
  [ -f "$f" ] || continue
  ffmpeg -hide_banner -nostdin -v error -y -i "$f" -c:v libx264 -preset veryfast -crf 26 -pix_fmt yuv420p -movflags +faststart "${f%.webm}.mp4" && rm -f "$f"
done
# MAC_JUDGE_BARS=1 (tests/l5/heartbeat-vm.sh, a quiet Mac's clone): a missed bar fails —
# except those in MAC_CLONE_ALARMS ("bar=limit,…"), which no VM holds (DEVIATIONS #49):
# missed against the product's bar they're said, as here; past the clone's own alarm (a
# gross regression, calibrated from the clone's runs) they fail too.
if [ "$code" -eq 2 ]; then
  missed=$(MAC_JUDGE_BARS="${MAC_JUDGE_BARS:-0}" MAC_CLONE_ALARMS="${MAC_CLONE_ALARMS:-}" node -e '
    const h = JSON.parse(require("fs").readFileSync(process.argv[1], "utf8"));
    const alarms = Object.fromEntries((process.env.MAC_CLONE_ALARMS || "").split(",").filter(Boolean).map((p) => p.split("=")).map(([k, v]) => [k, Number(v)]));
    const miss = Object.entries(h.bars).filter(([, b]) => !b.ok);
    const said = (l) => l.map(([k, b]) => `${k} ${b.value} (bar ${b.bar}${k in alarms ? `, clone alarm ${alarms[k]}` : ""})`).join(", ");
    const judged = process.env.MAC_JUDGE_BARS === "1" ? miss.filter(([k, b]) => !(k in alarms) || b.value > alarms[k]) : [];
    require("fs").writeFileSync(process.argv[2], said(miss.filter((m) => !judged.includes(m))) + "\n");
    console.log(said(judged));' "$MAC_OUT/heartbeat.json" "$MAC_OUT/bars.txt")
  if [ -n "$missed" ]; then
    echo "the whole path held; timing bars missed: $missed"
    exit 2
  fi
  echo "the whole path held; timing bars missed on this machine, for a Mac to decide: $(cat "$MAC_OUT/bars.txt")"
  exit 0
fi
exit "$code"
