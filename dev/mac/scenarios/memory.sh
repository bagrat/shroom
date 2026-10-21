# What shroom holds (DECISIONS B229), as Activity Monitor's Memory column counts it, in a
# build made the way a release is (optimized, `tauri build` without --debug) with the
# harness built in: at rest, after the Library and then Settings opened and closed, and
# idle after a 30-second take uploaded to the fake (app/test/take-drive.mjs memory) —
# first as shroom was before B229 (SHROOM_MEMORY_AS_BEFORE), then as built, on this one
# Mac. It measures and holds nothing to a bar: it fails only when the build or a drive does.
# The release-profile build from cold takes most of the time, and it replaces the dev
# build the job made: run it alone, or last.
# mac-limit: 3000
set -u
(cd app && bash ../tools/dev-cli.sh && npx tauri build --bundles app --features dev-harness --config src-tauri/tauri.dev-cli.conf.json) >"$MAC_OUT/build.log" 2>&1 \
  || { tail -n 40 "$MAC_OUT/build.log"; echo "NO: the release-profile build failed"; exit 1; }
# The grants and the window-picker approval were written for the dev build's path and its
# signature (the same throwaway identity signs both): the release build takes its place.
rm -rf "$MAC_APP" && ditto target/release/bundle/macos/shroom-dev.app "$MAC_APP" || { echo "NO: couldn't put the release build in place"; exit 1; }
# Both halves on this Mac, one after the other: as before B229, then as built.
SHROOM_MEMORY_AS_BEFORE=1 node app/test/take-drive.mjs memory >"$MAC_OUT/memory-before.json"
before=$?
node app/test/take-drive.mjs memory >"$MAC_OUT/memory.json"
after=$?
node -e 'const read = (f) => { try { return JSON.parse(require("fs").readFileSync(f, "utf8")).report; } catch { return {}; } };
  const [b, a] = process.argv.slice(1).map(read), mb = (r) => Object.fromEntries((r.facts?.memory ?? []).map((s) => [s.state, s]));
  const [B, A] = [mb(b), mb(a)];
  console.log("| state | before (MB, processes) | after (MB, processes) | change |\n|---|---|---|---|");
  for (const st of Object.keys(A).length ? Object.keys(A) : Object.keys(B)) {
    const x = B[st], y = A[st], d = x && y ? (y.footprint_mb - x.footprint_mb).toFixed(1) : "?";
    console.log(`| ${st} | ${x ? `${x.footprint_mb}, ${x.processes}` : "—"} | ${y ? `${y.footprint_mb}, ${y.processes}` : "—"} | ${d} |`);
  }
  for (const [n, r, S] of [["before", b, B], ["after", a, A]]) {
    for (const s of Object.values(S)) console.log(`${n}, ${s.state}: ${s.each.map((e) => `${e.name} ${e.footprint_mb}`).join(", ")} (readings ${s.readings_mb.join(", ")})`);
    console.log(`${n}, given back: ${(r.facts?.given_back ?? []).join(" / ") || "nothing"}`);
    if (r.error) console.log(`${n}, error: ${r.error}`);
  }' "$MAC_OUT/memory-before.json" "$MAC_OUT/memory.json"
[ "$before" = 0 ] && [ "$after" = 0 ] && echo "verdict: measured both ways (release-profile build); the table is above" || echo "NO: a memory drive didn't finish (before exit $before, after exit $after)"
[ "$before" = 0 ] && [ "$after" = 0 ]
