#!/bin/sh
# Moves shroom's version — every file that holds it, together (DECISIONS B61, B102).
#
# One version, held equal by the tests, in seven places:
#
#   app/src-tauri/tauri.conf.json   "version"            the bundle's, and the updater's
#   app/src-tauri/Cargo.toml        [package] version
#   app/package.json                "version"
#   Cargo.lock                      shroom-app's entry
#   package-lock.json               the app workspace's entry
#   worker/src/worker.js            VERSION              the page's (a space takes a page
#                                                        only when its VERSION is newer, B50)
#   worker/page-version.json        a new row            the page's fingerprint under that
#                                                        VERSION (worker/dev/page-fingerprint.mjs);
#                                                        rows are added; only an unpublished
#                                                        VERSION's row is re-stamped (B248)
#
# and docs/CHECKPOINT.md's update rehearsal, which names the version one patch above the tree's
# (its test, the_checkpoint_script_tells_the_truth_about_the_build, holds it to that).
#
#   tools/bump-version.sh <x.y.z>           move everything to x.y.z — it must be higher
#   tools/bump-version.sh <x.y.z-early.n>   an early build (DECISIONS B220): below x.y.z, above
#                                           x.y.z-early.(n-1) and the tree's own version
#   tools/bump-version.sh --check [x.y.z]   change nothing: do all seven agree (on x.y.z)?
#
# --check prints one plain line and fails when they don't; the tag workflow runs it with the
# tag's version, so a tag that isn't the tree's version never builds. It needs only sh, sed
# and awk. Moving the version needs node (for the page's fingerprint) and, when cargo is on
# the PATH, asks it that Cargo.lock still matches.
#
# It commits nothing and tags nothing — docs/RELEASING.md has the routine.
set -eu

ROOT=$(cd "$(dirname "$0")/.." && pwd)
fail() { echo "bump-version: $*" >&2; exit 1; }
usage() { echo "usage: tools/bump-version.sh <x.y.z> | --check [x.y.z]" >&2; exit 64; }

# Three whole numbers, no leading zeros, and for an early build `-early.<n>` (n ≥ 1, B220):
# the shape every reader of it takes.
shaped() {
  printf '%s\n' "$1" | grep -Eq '^(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)(-early\.[1-9][0-9]*)?$'
}

# A version's sort key, "major minor patch stage": an early build's stage is its n, a
# release's above every n — 0.1.25-early.1 < 0.1.25-early.2 < 0.1.25 < 0.1.26-early.1.
key() {
  printf '%s\n' "$1" | awk -F'[.-]' '{print $1, $2, $3, (NF > 3 ? $5 : 999999999)}'
}

# Where the version is, file by file — read exactly as the line is written there.
in_tauri() { sed -n 's/^  "version": "\(.*\)",$/\1/p' "$ROOT/app/src-tauri/tauri.conf.json"; }
in_cargo() { awk '/^\[/{s=$0} s=="[package]" && /^version = "/{gsub(/"/,"",$3); print $3; exit}' "$ROOT/app/src-tauri/Cargo.toml"; }
in_package() { sed -n 's/^  "version": "\(.*\)",$/\1/p' "$ROOT/app/package.json"; }
in_cargo_lock() { awk '/^name = "shroom-app"$/{f=1; next} f && /^version = "/{gsub(/"/,"",$3); print $3; exit}' "$ROOT/Cargo.lock"; }
in_package_lock() { awk '$0 == "    \"app\": {" {f=1; next} f && index($0, "      \"version\": \"") == 1 {gsub(/[",]/,"",$2); print $2; exit} f && index($0, "    }") == 1 {exit}' "$ROOT/package-lock.json"; }
in_worker() { sed -n 's/^export const VERSION = "\(.*\)";$/\1/p' "$ROOT/worker/src/worker.js"; }
# The newest row of the page's fingerprints (the last one: rows are only ever added).
in_page_rows() { sed -n 's/^  "\([0-9][0-9a-z.-]*\)": "[0-9a-f]\{64\}",\{0,1\}$/\1/p' "$ROOT/worker/page-version.json" | tail -n 1; }

# Every holder and what it says, one per line: "<file> <version>".
holders() {
  echo "app/src-tauri/tauri.conf.json $(in_tauri)"
  echo "app/src-tauri/Cargo.toml $(in_cargo)"
  echo "app/package.json $(in_package)"
  echo "Cargo.lock $(in_cargo_lock)"
  echo "package-lock.json $(in_package_lock)"
  echo "worker/src/worker.js $(in_worker)"
  echo "worker/page-version.json $(in_page_rows)"
}

# All seven say <version> — or one plain line naming the first that doesn't.
agree() {
  holders | while read -r file said; do
    [ "$said" = "$1" ] || { echo "$file says ${said:-nothing readable}, not $1"; exit 1; }
  done
}

case "${1-}" in
  --check)
    [ $# -le 2 ] || usage
    want=${2-$(in_tauri)}
    shaped "$want" || fail "'$want' isn't a version — it looks like 1.2.3, or 1.2.3-early.1"
    why=$(agree "$want") || fail "$why — every version file moves together: tools/bump-version.sh $want"
    echo "$want"
    exit 0
    ;;
  ""|-*) usage ;;
esac
[ $# -eq 1 ] || usage
new=$1
shaped "$new" || fail "'$new' isn't a version — it looks like 1.2.3, or 1.2.3-early.1"

old=$(in_tauri)
shaped "$old" || fail "couldn't read the current version from app/src-tauri/tauri.conf.json"
why=$(agree "$old") || fail "the tree doesn't agree on its version yet ($why) — nothing changed"
higher=$(printf '%s\n%s\n' "$(key "$old")" "$(key "$new")" | awk 'NR==1{a=$1; b=$2; c=$3; d=$4} NR==2{print ($1>a || ($1==a && ($2>b || ($2==b && ($3>c || ($3==c && $4>d)))))) ? "yes" : "no"}')
# No move down, not even to an early build of an unreleased x.y.z: the tree already holds
# x.y.z's page row, and the page's VERSION is its newest row (B220, B248). An early build
# leads to the version after the tree's: 0.1.33-early.1 from an unreleased 0.1.32.
[ "$higher" = yes ] || fail "$new isn't higher than $old — a version only goes up (an installed shroom takes a newer one only)"
command -v node >/dev/null 2>&1 || fail "needs node, for the page's fingerprint"

# The rewrite: each file's own line, exactly once, or nothing is written anywhere.
node --input-type=module - "$ROOT" "$old" "$new" <<'JS'
import { readFileSync, writeFileSync, existsSync } from "node:fs";
import { pathToFileURL } from "node:url";
const [root, old, next] = process.argv.slice(2);
const at = (f) => `${root}/${f}`;
const esc = (s) => s.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
const edits = [];
// One line of a file, found exactly once, rewritten.
const line = (file, pattern, to) => {
  const text = readFileSync(at(file), "utf8");
  const found = text.match(new RegExp(pattern, "gm")) ?? [];
  if (found.length !== 1) throw new Error(`${file}: expected its version line once, found it ${found.length} times`);
  edits.push([file, text.replace(new RegExp(pattern, "m"), to)]);
};
const o = esc(old);
line("app/src-tauri/tauri.conf.json", `^  "version": "${o}",$`, `  "version": "${next}",`);
line("app/src-tauri/Cargo.toml", `^(\\[package\\]\\nname = "shroom-app"\\n)version = "${o}"$`, `$1version = "${next}"`);
line("app/package.json", `^  "version": "${o}",$`, `  "version": "${next}",`);
line("Cargo.lock", `^(name = "shroom-app"\\n)version = "${o}"$`, `$1version = "${next}"`);
line("package-lock.json", `^(    "app": \\{\\n      "name": "shroom-app",\\n)      "version": "${o}",$`, `$1      "version": "${next}",`);
line("worker/src/worker.js", `^export const VERSION = "${o}";$`, `export const VERSION = "${next}";`);
const rowsFile = "worker/page-version.json";
const rows = readFileSync(at(rowsFile), "utf8");
const table = JSON.parse(rows);
if (table[next]) throw new Error(`${rowsFile} already has a row for ${next}`);
if (!/"\n\}\s*$/.test(rows)) throw new Error(`${rowsFile}: its last row isn't where a new one can follow`);
for (const [file, text] of edits) writeFileSync(at(file), text);

// The page's row: the fingerprint of the page as it is now, VERSION and all — added, never edited.
const { fingerprint } = await import(pathToFileURL(at("worker/dev/page-fingerprint.mjs")).href);
const print = fingerprint();
if (Object.entries(table).some(([k, v]) => k !== "_" && v === print)) throw new Error(`${rowsFile}: another version already shipped this very page`);
const grown = rows.replace(/"\n\}\s*$/, `",\n  "${next}": "${print}"\n}\n`);
if (grown === rows || JSON.parse(grown)[next] !== print) throw new Error(`${rowsFile}: couldn't add the row for ${next}`);
writeFileSync(at(rowsFile), grown);

// The checkpoint's rehearsal names the release one patch above the tree's (an early build's
// own release counts: 0.1.25-early.1 → 0.1.26, as 0.1.25 does).
const patch = (v) => v.replace(/-early\.\d+$/, "").replace(/\d+$/, (n) => String(Number(n) + 1));
if (existsSync(at("docs/CHECKPOINT.md"))) {
  const doc = readFileSync(at("docs/CHECKPOINT.md"), "utf8");
  const moved = doc.replace(new RegExp(`(?<![0-9.])${esc(patch(old))}(?![0-9]|\\.[0-9])`, "g"), patch(next));
  if (moved !== doc) writeFileSync(at("docs/CHECKPOINT.md"), moved);
}
JS

why=$(agree "$new") || fail "after the move, $why — look at git diff before going on"
if command -v cargo >/dev/null 2>&1; then
  said=$(cd "$ROOT" && cargo metadata --locked --offline --format-version 1 2>&1 >/dev/null) \
    || { printf '%s\n' "$said" | tail -n 5 >&2; fail "cargo doesn't take Cargo.lock as it is now (cargo metadata --locked --offline) — look at git diff before going on"; }
fi

echo "shroom $old → $new"
holders | while read -r file said; do echo "  $file"; done
echo "  docs/CHECKPOINT.md (the update rehearsal: $(printf '%s' "$new" | awk -F. '{print $1"."$2"."$3+1}'))"
echo
echo "Next: run the tests, then commit, tag v$new and push — docs/RELEASING.md, “Cutting a release”."
