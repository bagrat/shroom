# What shroom holds (DECISIONS B229), as Activity Monitor's Memory column counts it, in a
# build made the way a release is (optimized, `tauri build` without --debug) with the
# harness built in: at rest, after the Library and then Settings opened and closed, and
# idle after a 30-second take uploaded to the fake (app/test/take-drive.mjs memory). It
# measures and holds nothing to a bar: it fails only when the build or the drive does.
# The release-profile build from cold takes most of the time, and it replaces the dev
# build the job made: run it alone, or last.
# mac-limit: 3000
set -u
(cd app && bash ../tools/dev-cli.sh && npx tauri build --bundles app --features dev-harness --config src-tauri/tauri.dev-cli.conf.json) >"$MAC_OUT/build.log" 2>&1 \
  || { tail -n 40 "$MAC_OUT/build.log"; echo "NO: the release-profile build failed"; exit 1; }
# The grants and the window-picker approval were written for the dev build's path and its
# signature (the same throwaway identity signs both): the release build takes its place.
rm -rf "$MAC_APP" && ditto target/release/bundle/macos/shroom-dev.app "$MAC_APP" || { echo "NO: couldn't put the release build in place"; exit 1; }
node app/test/take-drive.mjs memory >"$MAC_OUT/memory.json"
rc=$?
node -e 'const r = JSON.parse(require("fs").readFileSync(process.argv[1], "utf8")).report, m = r.facts?.memory ?? [];
  console.log("| state | Activity Monitor (MB) | resident (MB) | processes |\n|---|---|---|---|");
  for (const s of m) console.log(`| ${s.state} | ${s.footprint_mb} | ${s.resident_mb} | ${s.processes} |`);
  for (const s of m) console.log(`${s.state}: ${s.each.map((e) => `${e.name} ${e.footprint_mb}`).join(", ")}`);
  if (r.error) console.log("error: " + r.error);' "$MAC_OUT/memory.json"
[ "$rc" = 0 ] && echo "verdict: measured (release-profile build); the table is above" || echo "NO: the memory drive didn't finish (exit $rc)"
exit "$rc"
