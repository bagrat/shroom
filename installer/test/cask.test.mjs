// The Homebrew cask (installer/homebrew/): what it pins, what it removes, and the script
// that fills in each release's version and fingerprint for the tap.
// Run: node --test installer/test/cask.test.mjs
import { spawnSync } from "node:child_process";
import { mkdtempSync, readFileSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { describe, it } from "node:test";
import assert from "node:assert/strict";

const ROOT = new URL("../..", import.meta.url).pathname;
const CASK = join(ROOT, "installer/homebrew/shroom.rb");
const RENDER = join(ROOT, "installer/homebrew/render-cask.sh");
const cask = readFileSync(CASK, "utf8");
const work = mkdtempSync(join(tmpdir(), "shroom-cask-"));

const render = (manifest) => {
  const path = join(work, `manifest-${Math.random().toString(36).slice(2)}.txt`);
  writeFileSync(path, manifest);
  return spawnSync("sh", [RENDER, path], { encoding: "utf8" });
};
const SUM = "ab".repeat(32);

describe("the cask", () => {
  // The preflight step's shell, as Homebrew hands it to /bin/sh (a heredoc Ruby leaves as is),
  // with the staged copy as its one argument.
  const step = cask.split("  preflight_steps do\n    run \"/bin/sh\", args: [\"-c\", <<~'SH', \"shroom-check\", \"{{staged_path}}/shroom.app\"]\n")[1].split("\n    SH\n")[0].replace(/^ {6}/gm, "");

  it("pins the release certificate the installer and tools/release-identity.sha1 pin (DECISIONS B125)", () => {
    const pin = readFileSync(join(ROOT, "tools/release-identity.sha1"), "utf8").trim();
    const installer = readFileSync(join(ROOT, "installer/install.sh"), "utf8");
    assert.ok(installer.includes(`\nRELEASE_LEAF=${pin}\n`));
    assert.ok(step.includes(`\nleaf=${pin}\n`), "the cask's leaf is the release certificate's");
    assert.ok(step.includes('-R "=identifier \\"am.shroom\\" and certificate leaf = H\\"$leaf\\"" "$app"'), step);
    assert.ok(step.startsWith("app=$1\n"), "the copy it checks is the staged one, before anything is placed");
  });

  it("checks the signature before it lifts the quarantine, and fails the install when it doesn't verify", () => {
    const verify = step.indexOf("/usr/bin/codesign");
    const lift = step.indexOf('/usr/bin/xattr -dr com.apple.quarantine "$app"');
    assert.ok(verify >= 0 && lift > verify, step);
    assert.match(step, /then\n\s+echo ".*" >&2\n\s+exit 1\n\s*fi\n/);
    assert.equal(spawnSync("sh", ["-n", "-c", step]).status, 0, "it is shell that parses");
  });

  it("says where to tell us as the installer's signature line does", () => {
    const installer = readFileSync(join(ROOT, "installer/install.sh"), "utf8");
    const tell = installer.match(/^SIGNATURE=".*(tell us: .*)"$/m)[1];
    const said = step.match(/echo "(.*)" >&2/)[1];
    assert.ok(said.startsWith("shroom's signature didn't verify, so nothing was installed.") && said.endsWith(tell), said);
  });

  it("is Apple silicon only, macOS 14 and newer, and leaves updates to the app (DECISIONS B31)", () => {
    assert.match(cask, /^  depends_on arch: :arm64$/m);
    assert.match(cask, /^  depends_on macos: :sonoma$/m);
    assert.match(cask, /^  auto_updates true$/m);
    const installer = readFileSync(join(ROOT, "installer/install.sh"), "utf8");
    assert.ok(installer.includes("\nMIN_MACOS=14\n"), "Sonoma is the installer's floor too");
  });

  it("places shroom where the install line and the updater keep it", () => {
    assert.match(cask, /^  app "shroom\.app", target: "~\/Applications\/shroom\.app"$/m);
  });

  // Bagrat, 2026-10-18: the `shroom` command is always installed — Homebrew links the one
  // inside the app, the same file the install line links into ~/.local/bin.
  it("links the app's shroom command into Homebrew's bin, the same file the install line links", () => {
    assert.match(cask, /^  binary "#\{Dir\.home\}\/Applications\/shroom\.app\/Contents\/MacOS\/shroom-cli", target: "shroom"$/m);
    const installer = readFileSync(join(ROOT, "installer/install.sh"), "utf8");
    assert.ok(installer.includes('target="$APP/Contents/MacOS/shroom-cli"'));
    assert.ok(installer.includes('APP="$HOME/Applications/shroom.app"'));
  });

  it("downloads the release's own zip, the name manifest.txt gives it", () => {
    assert.ok(cask.includes('url "https://github.com/bagrat/shroom/releases/download/v#{version}/shroom-#{version}-arm64.zip"'));
  });

  it("zaps exactly what install.sh --uninstall removes beside the app and ~/.shroom (DECISIONS B135)", () => {
    const installer = readFileSync(join(ROOT, "installer/install.sh"), "utf8");
    const rm = installer.split('  rm -rf "$APP" "$dir" ')[1].split("\n  say")[0];
    const theirs = [...rm.matchAll(/"\$lib\/([^"]+)"/g)].map((m) => `~/Library/${m[1].replaceAll("$id", "am.shroom")}`);
    const zap = cask.split("  zap trash: [\n")[1].split("\n  ]")[0];
    const ours = [...zap.matchAll(/^\s+"([^"]+)",$/gm)].map((m) => m[1]);
    assert.deepEqual([...ours].sort(), ["~/.shroom", ...theirs].sort());
    assert.match(cask, /^  uninstall quit: "am\.shroom"$/m);
  });

  it("is Ruby that parses", { skip: spawnSync("ruby", ["-v"]).status !== 0 && "no ruby here" }, () => {
    const r = spawnSync("ruby", ["-c", CASK], { encoding: "utf8" });
    assert.equal(r.status, 0, r.stderr);
  });
});

describe("render-cask.sh", () => {
  it("fills in the manifest's version and fingerprint, and changes nothing else", () => {
    const r = render(`version=1.2.3\nmacos=14\narm64=shroom-1.2.3-arm64.zip ${SUM.toUpperCase()}\n`);
    assert.equal(r.status, 0, r.stderr);
    const want = cask.replace(/^  version "[0-9.]+"$/m, '  version "1.2.3"').replace(/^  sha256 "[0-9a-f]+"$/m, `  sha256 "${SUM}"`);
    assert.notEqual(want, cask);
    assert.equal(r.stdout, want);
  });

  it("gives back the cask as it is for the release it names", () => {
    const version = cask.match(/^  version "([0-9.]+)"$/m)[1];
    const sum = cask.match(/^  sha256 "([0-9a-f]{64})"$/m)[1];
    const r = render(`version=${version}\nmacos=14\narm64=shroom-${version}-arm64.zip ${sum}\n`);
    assert.equal(r.status, 0, r.stderr);
    assert.equal(r.stdout, cask);
  });

  for (const [what, manifest] of [
    ["no version", `macos=14\narm64=shroom-1.2.3-arm64.zip ${SUM}\n`],
    ["a version of another shape", `version=1.2.3.4\nmacos=14\narm64=shroom-1.2.3.4-arm64.zip ${SUM}\n`],
    ["no arm64 file", "version=1.2.3\nmacos=14\n"],
    ["a file the cask's url wouldn't name", `version=1.2.3\nmacos=14\narm64=other-1.2.3-arm64.zip ${SUM}\n`],
    ["a short fingerprint", `version=1.2.3\nmacos=14\narm64=shroom-1.2.3-arm64.zip ${SUM.slice(2)}\n`],
    ["a fingerprint that isn't hex", `version=1.2.3\nmacos=14\narm64=shroom-1.2.3-arm64.zip ${"zz".repeat(32)}\n`],
  ]) {
    it(`refuses a manifest with ${what}, printing nothing`, () => {
      const r = render(manifest);
      assert.notEqual(r.status, 0);
      assert.equal(r.stdout, "");
      assert.match(r.stderr, /^render-cask: /);
    });
  }
});
