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
  it("pins the release certificate the installer and tools/release-identity.sha1 pin (DECISIONS B125)", () => {
    const pin = readFileSync(join(ROOT, "tools/release-identity.sha1"), "utf8").trim();
    const installer = readFileSync(join(ROOT, "installer/install.sh"), "utf8");
    assert.ok(installer.includes(`\nRELEASE_LEAF=${pin}\n`));
    assert.ok(cask.includes(`    leaf = "${pin}"\n`), "the cask's leaf is the release certificate's");
    assert.ok(cask.includes('"=identifier \\"am.shroom\\" and certificate leaf = H\\"#{leaf}\\""'), "as am.shroom, by that leaf");
  });

  it("checks the signature before it lifts the quarantine, and takes the app away when it doesn't verify", () => {
    const post = cask.split("  postflight do\n")[1].split("\n  end\n")[0];
    const verify = post.indexOf("/usr/bin/codesign");
    const lift = post.indexOf('"/usr/bin/xattr", args: ["-dr", "com.apple.quarantine", placed]');
    assert.ok(verify >= 0 && lift > verify, post);
    assert.match(post, /unless signed\n\s+FileUtils\.rm_rf\(placed\)\n\s+raise /);
  });

  it("says where to tell us as the installer's signature line does", () => {
    const installer = readFileSync(join(ROOT, "installer/install.sh"), "utf8");
    const tell = installer.match(/^SIGNATURE=".*(tell us: .*)"$/m)[1];
    const said = cask.split("  postflight do\n")[1].split("\n  end\n")[0].match(/raise "(.*)" \\\n\s+"(.*)"/);
    assert.ok(`${said[1]}${said[2]}`.endsWith(tell), `${said[1]}${said[2]}`);
  });

  it("is Apple silicon only, macOS 14 and newer, and leaves updates to the app (DECISIONS B31)", () => {
    assert.match(cask, /^  depends_on arch: :arm64$/m);
    assert.match(cask, /^  depends_on macos: ">= :sonoma"$/m);
    assert.match(cask, /^  auto_updates true$/m);
    const installer = readFileSync(join(ROOT, "installer/install.sh"), "utf8");
    assert.ok(installer.includes("\nMIN_MACOS=14\n"), "Sonoma is the installer's floor too");
  });

  it("places shroom where the install line and the updater keep it", () => {
    assert.match(cask, /^  app "shroom\.app", target: "~\/Applications\/shroom\.app"$/m);
    assert.ok(cask.includes('placed = File.expand_path("~/Applications/shroom.app")'));
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
