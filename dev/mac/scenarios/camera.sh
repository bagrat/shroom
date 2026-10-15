# A camera on a Mac that has none (dev/mac/camera.sh: OBS's virtual camera on a test
# pattern), then the dev app's own check of it: the camera it picks, three seconds of its
# frames, and a three-second take through the real path, decoded and deleted. Its frames
# must be the pattern's bright bars, not OBS's dark "not started" picture (brightness ~45).
bash dev/mac/camera.sh up || exit 1
out="$MAC_OUT/camera-check.json"
open -W -n "$MAC_APP" --args --camera-check "$out" record
node -e 'const r = JSON.parse(require("fs").readFileSync(process.argv[1], "utf8")), s = r.sample || {};
  console.log(JSON.stringify(r, null, 1));
  console.log(`camera: ${r.chosen || "none"} (access ${JSON.stringify(r.access)}); ${s.delivered || 0} frames at ${s.fps || 0} fps, ${s.width}x${s.height}, brightness ${Math.round(s.mean_luma || 0)}; take ${JSON.stringify(r.recorded)}${r.error ? "; error: " + r.error : ""}`);
  process.exit(r.chosen && !r.error && s.delivered > 0 && s.mean_luma > 80 && r.recorded && !r.recorded.error ? 0 : 1)' "$out"
