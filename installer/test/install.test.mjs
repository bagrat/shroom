// The installer (Region 1: installing, updating, and its refusals) against a local
// stand-in for GitHub releases, with this Mac's facts and the app's quit stubbed.
// Run: node --test installer/test/
import { execFileSync, spawn, spawnSync } from "node:child_process";
import { createHash } from "node:crypto";
import { chmodSync, existsSync, mkdirSync, mkdtempSync, readFileSync, readdirSync, readlinkSync, lstatSync, rmSync, statSync, symlinkSync, writeFileSync } from "node:fs";
import { createServer } from "node:http";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { after, before, describe, it } from "node:test";
import assert from "node:assert/strict";

const ROOT = new URL("../..", import.meta.url).pathname;
const SCRIPT = join(ROOT, "installer/install.sh");
const machine = JSON.parse(readFileSync(join(ROOT, "spec/machine.json"), "utf8"));
const said = (id) => machine.states[id].said.map((l) => l.text);

// Lines the spec doesn't print (DECISIONS' copy table): the installer's own words.
const NO_BUILD = "shroom doesn't run on this kind of Mac yet — Apple silicon only, for now.";
const DIDNT_QUIT = "shroom didn't close — quit it from the menu bar, then run this again.";
const uninstall = said("uninstall-sh");
const NO_ROOM = "Couldn't place shroom on this Mac — check there's room for it, then run the same command again.";
const DIDNT_OPEN = "shroom is in ~/Applications but didn't open — open it from there.";
const NOT_A_MAC = "shroom runs on a Mac — run this in Terminal on yours.";
const NOT_ALL_THERE = "This release isn't all there on our side, so nothing was installed. Tell us and we'll fix it: https://github.com/bagrat/shroom/issues/new — no GitHub? hi@shroom.am works too.";
const SIGNATURE = "shroom's signature didn't verify, so nothing was installed. Run the same command in Terminal — if it repeats, tell us: https://github.com/bagrat/shroom/issues/new — no GitHub? hi@shroom.am works too.";
// The early channel with no early build out (B220; Bagrat OK'd the words, 2026-10-18).
const NO_EARLY = "No early build right now, so you're on the latest release — early builds will come as updates.";
const RESTRICTED = "This looks like it's running somewhere restricted, so nothing was installed — run the same command in Terminal.";
const UNCHANGED = said("install-refused-midtake")[1];
const NET_DROP = said("install-net-drop")[0];
const CHECKSUM = said("install-checksum-fail")[0];

const scratch = mkdtempSync(join(tmpdir(), "shroom-installer-"));
const stubs = join(scratch, "bin");
mkdirSync(stubs);
const stub = (name, body) => { writeFileSync(join(stubs, name), `#!/bin/sh\n${body}\n`); chmodSync(join(stubs, name), 0o755); };
// A machine that isn't a Mac: `uname -s` says so, and there's no sw_vers to ask.
stub("sw_vers", `if [ "\${FAKE_MACOS:-}" = none ]; then echo "sh: sw_vers: command not found" >&2; exit 127; fi
echo "\${FAKE_MACOS:-15.1}"`);
stub("uname", `case "\${1:-}" in
  -m) echo "\${FAKE_ARCH:-arm64}" ;;
  -s) echo "\${FAKE_OS:-Darwin}" ;;
  *) exec /usr/bin/uname "$@" ;;
esac`);
// Apple silicon answers 1 — even to a Terminal under Rosetta; an Intel Mac has no such key.
stub("sysctl", `if [ "$*" = "-n hw.optional.arm64" ]; then
  if [ "\${FAKE_ARM64:-1}" = none ]; then echo "sysctl: unknown oid 'hw.optional.arm64'" >&2; exit 1; fi
  echo "\${FAKE_ARM64:-1}"
else exec /usr/sbin/sysctl "$@"; fi`);
// macOS refusing to open the app answers the way LaunchServices does: its own error text.
stub("open", `echo "open $*" >> "$FAKE_LOG"
if [ -n "\${FAKE_OPEN_FAIL:-}" ]; then echo "The application cannot be opened because its executable is missing." >&2; echo "LSOpenURLsWithRole() failed with error -10810 for the file $2." >&2; exit 1; fi`);
stub("osascript", `id=$(printf '%s' "$*" | sed -n 's/.*application id "\\([^"]*\\)".*/\\1/p')
case "$*" in
  *"is running"*) if [ -e "$FAKE_RUNNING" ]; then echo true; else echo false; fi ;;
  *"to quit"*) echo "quit $id" >> "$FAKE_LOG"; [ -n "\${FAKE_STUCK:-}" ] || rm -f "$FAKE_RUNNING" ;;
esac`);
// A move this Mac refuses (a full disk, a locked folder): fails, with the error text mv
// would print, for any source that matches FAKE_MV_FAIL — or a sandbox's, with FAKE_MV_ERR.
stub("mv", `for a in "$@"; do case "$a" in -*) ;; *) src=$a; break ;; esac; done
for p in \${FAKE_MV_FAIL:-}; do case "$src" in *"$p"*) echo "mv: rename $src: \${FAKE_MV_ERR:-Permission denied}" >&2; exit 1 ;; esac; done
exec /bin/mv "$@"`);
stub("ditto", `if [ -n "\${FAKE_DITTO_FAIL:-}" ] && [ "$1" = -x ]; then echo "ditto: Couldn't write: \${FAKE_DITTO_FAIL}" >&2; exit 1; fi
exec /usr/bin/ditto "$@"`);
// The fixtures carry no real signature: a signature check passes unless the run says it
// fails — shroom's alone (FAKE_CODESIGN_FAIL=1), or every one, Calculator's too (=all),
// the way a sandbox that keeps codesign from the trust service fails them.
// =installed fails only the copy already in ~/Applications (broken, or changed since).
stub("codesign", `for a in "$@"; do last=$a; done
case "\${FAKE_CODESIGN_FAIL:-}:$last" in
  :*|1:*/Calculator.app) exit 0 ;;
  installed:*/Applications/shroom.app) ;;
  installed:*) exit 0 ;;
esac
echo "$last: a sealed resource is missing or invalid" >&2; exit 1`);

// What an uninstall asks of macOS: LaunchServices (the app registered, then forgotten),
// the permission resets and the app's preferences — each only logged, never this Mac's.
stub("lsregister", `echo "lsregister $*" >> "$FAKE_LOG"`);
stub("tccutil", `echo "tccutil $*" >> "$FAKE_LOG"; echo "Successfully reset $2 approval status for $3"`);
stub("defaults", `echo "defaults $*" >> "$FAKE_LOG"`);

// A release: shroom.app zipped the way releases are (ditto keeps signatures intact).
function release(version) {
  const dir = mkdtempSync(join(scratch, "rel-"));
  mkdirSync(join(dir, "shroom.app/Contents/MacOS"), { recursive: true });
  writeFileSync(join(dir, "shroom.app/Contents/version"), version);
  // The `shroom` command the app carries (spec N62, DECISIONS B217).
  writeFileSync(join(dir, "shroom.app/Contents/MacOS/shroom-cli"), "#!/bin/sh\necho shroom\n");
  chmodSync(join(dir, "shroom.app/Contents/MacOS/shroom-cli"), 0o755);
  const zip = join(dir, `shroom-${version}-arm64.zip`);
  execFileSync("ditto", ["-c", "-k", "--keepParent", join(dir, "shroom.app"), zip]);
  return { file: `shroom-${version}-arm64.zip`, bytes: readFileSync(zip), version };
}
const v120 = release("1.2.0");
// A release zip that unpacks to something other than shroom.app.
const v120hollow = (() => {
  const dir = mkdtempSync(join(scratch, "rel-"));
  mkdirSync(join(dir, "other.app/Contents"), { recursive: true });
  const zip = join(dir, v120.file);
  execFileSync("ditto", ["-c", "-k", "--keepParent", join(dir, "other.app"), zip]);
  return { ...v120, bytes: readFileSync(zip) };
})();
const sha = (b) => createHash("sha256").update(b).digest("hex");

// `mode` bends the stand-in: a manifest's lines, an asset's answer, a dropped transfer.
let mode = {};
let hits = 0;
/** @type {string[]} */
let counted = []; // what the stand-in for shroom.am/download was asked
const manifest = () => {
  if (mode.manifest) return mode.manifest;
  const lines = [`version=${v120.version}`, `macos=${mode.floor ?? 14}`];
  if (!mode.noArm) lines.push(`arm64=${v120.file} ${mode.badSum ? "0".repeat(64) : sha(served().bytes)}`);
  return lines.join("\n") + "\n";
};
const served = () => (mode.hollow ? v120hollow : v120);
const server = createServer((req, res) => {
  hits++;
  // shroom.am/download's stand-in (DECISIONS B228): counts the ask, then sends curl on.
  if (req.url.startsWith("/counted/")) {
    counted.push(req.url);
    res.writeHead(302, { location: `/download/${req.url.slice("/counted/".length).split("?")[0]}` });
    return res.end();
  }
  // The early channel's pointer (DECISIONS B220): served only when a test asks for it.
  if (req.url === "/download/early/manifest.txt" && mode.early) {
    res.end(mode.earlyManifest ?? manifest());
  } else if (req.url === "/latest/download/manifest.txt") {
    if (mode.manifestStatus) { res.statusCode = mode.manifestStatus; return res.end(); }
    res.end(manifest());
  } else if (new RegExp(`^/download/v\\d+\\.\\d+\\.\\d+-early\\.\\d+/${v120.file.replace(/\./g, "\\.")}$`).test(req.url)) {
    res.end(v120.bytes); // an early build's tag (DECISIONS B220), the same zip
  } else if (req.url === `/download/v${v120.version}/${v120.file}`) {
    if (mode.assetStatus) { res.statusCode = mode.assetStatus; return res.end(); }
    if (mode.drop) {
      res.writeHead(200, { "content-length": v120.bytes.length });
      res.write(v120.bytes.subarray(0, 100));
      setTimeout(() => res.destroy(), 20);
    } else res.end(served().bytes);
  } else { res.statusCode = 404; res.end(); }
});
let base;
before(async () => { await new Promise((r) => server.listen(0, "127.0.0.1", r)); base = `http://127.0.0.1:${server.address().port}`; });
after(() => { server.close(); rmSync(scratch, { recursive: true, force: true }); });

async function run({ args = [], env = {}, setup, with: m = {} } = {}) {
  mode = m;
  const home = mkdtempSync(join(scratch, "home-"));
  const log = join(home, "log");
  writeFileSync(log, "");
  const running = join(home, "running");
  if (setup) setup(home, running);
  hits = 0;
  counted = [];
  // Asynchronous: the stand-in release server answers from this same process.
  const child = spawn("sh", [SCRIPT, ...args], {
    env: { PATH: `${stubs}:/usr/bin:/bin:/usr/sbin:/sbin`, HOME: home, SHROOM_RELEASES: base, FAKE_LOG: log, FAKE_RUNNING: running, ...env },
  });
  let stdout = "";
  let stderr = "";
  child.stdout.on("data", (d) => (stdout += d));
  child.stderr.on("data", (d) => (stderr += d));
  const code = await new Promise((resolve) => child.on("close", resolve));
  mode = {};
  for (const d of [home, join(home, "Applications")]) if (existsSync(d)) chmodSync(d, 0o755);
  const app = join(home, "Applications/shroom.app/Contents/version");
  return {
    code, out: stdout.trim().split("\n"), err: stderr, log: readFileSync(log, "utf8"), hits, home,
    installed: existsSync(app) ? readFileSync(app, "utf8") : null,
    leftovers: readdirSync(home).filter((n) => n.startsWith(".shroom-install")),
  };
}
const installOld = (home, id) => {
  mkdirSync(join(home, "Applications/shroom.app/Contents"), { recursive: true });
  writeFileSync(join(home, "Applications/shroom.app/Contents/version"), "1.1.0");
  if (id) {
    const plist = join(home, "Applications/shroom.app/Contents/Info.plist");
    execFileSync("plutil", ["-create", "xml1", plist]);
    execFileSync("plutil", ["-insert", "CFBundleIdentifier", "-string", id, plist]);
  }
};
const runningOld = (home, running) => { installOld(home); writeFileSync(running, ""); };
// The release's own version already here, named as the release names it (Info.plist).
const installLatest = (home, id = "am.shroom", version = v120.version) => {
  installOld(home, id);
  const plist = join(home, "Applications/shroom.app/Contents/Info.plist");
  execFileSync("plutil", ["-insert", "CFBundleShortVersionString", "-string", version, plist]);
  writeFileSync(join(home, "Applications/shroom.app/Contents/version"), `${version} (here before)`);
};
const runningLatest = (home, running) => { installLatest(home); writeFileSync(running, ""); };
const recordingNow = (home, dir = ".shroom") => {
  mkdirSync(join(home, dir), { recursive: true });
  writeFileSync(join(home, dir, "status.json"), JSON.stringify({ v: 1, pid: process.pid, take_open: true }));
};

// Law 15: what reaches the Terminal is the said-lines and curl's own bar — never curl's
// error lines, its retry warnings, an error code or a tool's complaint.
const BAR = /^[#O=\-\s\d.%]*$/;
const quiet = (r) => {
  assert.match(r.err, BAR, `stderr holds only the progress bar: ${JSON.stringify(r.err)}`);
  for (const line of r.out) assert.doesNotMatch(line, /curl|mv:|mkdir|ditto|LSOpen|cannot be opened|\(\d+\)|-\d{3,}|Warning/, line);
};

describe("installing", () => {
  it("downloads, verifies, places and opens — said in the spec's words", async () => {
    const r = await run();
    assert.equal(r.code, 0);
    assert.deepEqual(r.out, ["Getting shroom…", "verified ✓", "Installed — opening shroom now."]);
    assert.equal(said("installing")[0], "Getting shroom… verified ✓"); // printed around curl's own progress bar
    assert.equal(r.out[2], said("installing")[1]);
    assert.equal(r.installed, "1.2.0");
    assert.match(r.err, /#/, "curl's own bar shows");
    quiet(r);
    assert.deepEqual(r.leftovers, [], "the temporary folder goes away");
  });

  // Bagrat, 2026-10-18: the `shroom` command is always installed — in the home folder,
  // as everything else is.
  it("links the shroom command into ~/.local/bin and puts that folder on a new Terminal's PATH, once", async () => {
    const MARK = "# shroom's command line (shroom.am) — run shroom --help";
    const r = await run({ env: { SHELL: "/bin/zsh" }, setup: (home) => writeFileSync(join(home, ".zprofile"), "export FOO=1\n") });
    assert.equal(r.code, 0);
    assert.equal(readlinkSync(join(r.home, ".local/bin/shroom")), join(r.home, "Applications/shroom.app/Contents/MacOS/shroom-cli"));
    const profile = readFileSync(join(r.home, ".zprofile"), "utf8");
    assert.ok(profile.startsWith("export FOO=1\n"), "the profile keeps what was there");
    assert.equal(profile.split(MARK).length, 2, "one marked line");
    // After the system's folders, never before them: nothing already sitting in ~/.local/bin
    // gets to stand in for a system command (the security review, 2026-10-18).
    assert.match(profile, /\nexport PATH="\$PATH:\$HOME\/\.local\/bin"\n/);
    assert.deepEqual(r.out, ["Getting shroom…", "verified ✓", "Installed — opening shroom now."], "said nothing more");
  });

  it("never replaces a file of someone else's named shroom", async () => {
    const r = await run({ setup: (home) => { mkdirSync(join(home, ".local/bin"), { recursive: true }); writeFileSync(join(home, ".local/bin/shroom"), "mine"); } });
    assert.equal(r.code, 0);
    assert.equal(readFileSync(join(r.home, ".local/bin/shroom"), "utf8"), "mine");
  });

  it("tells the app it was just installed (its first launch)", async () => {
    const r = await run();
    assert.match(r.log, /^open .*\/Applications\/shroom\.app --args --relaunched=install\n$/);
  });

  it("hands an invite to the app by file, never on its command line", async () => {
    const r = await run({ args: ["--join", "https://v.myshroom.workers.dev/join/abc"] });
    assert.match(r.log, /^open .*shroom\.app --args --relaunched=install --invite\n$/);
    assert.ok(!r.log.includes("join/abc"), "the link is not in the app's arguments");
    const file = join(r.home, ".shroom/invite");
    assert.equal(statSync(file).mode & 0o777, 0o600);
    const invite = JSON.parse(readFileSync(file, "utf8"));
    assert.equal(invite.link, "https://v.myshroom.workers.dev/join/abc");
    assert.match(invite.arrived, /^\d{4}-\d\d-\d\dT\d\d:\d\d:\d\dZ$/);
  });

  it("with --agent, opens shroom as the mushroom only, for the agent's shroom setup (N88)", async () => {
    const r = await run({ args: ["--agent"] });
    assert.equal(r.code, 0);
    assert.deepEqual(r.out, ["Getting shroom…", "verified ✓", "Installed — opening shroom now."], "the same lines: Installed is the agent's cue");
    assert.match(r.log, /^open .*\/Applications\/shroom\.app --args --relaunched=agent\n$/);
    const both = await run({ args: ["--early", "--agent"], with: { early: true, earlyManifest: `version=1.3.0-early.1\nmacos=14\narm64=${v120.file} ${sha(v120.bytes)}\n` } });
    assert.match(both.log, /--relaunched=agent\n$/, "either order with --early");
    const invited = await run({ args: ["--agent", "--join", "https://v.myshroom.workers.dev/join/abc"] });
    assert.match(invited.log, /--relaunched=install --invite\n$/, "an invite's window is the member lane's");
  });

  it("opens shroom plainly for something that isn't a link's shape", async () => {
    const r = await run({ args: ["--join", 'x","evil":"1'] });
    assert.match(r.log, /^open .*shroom\.app --args --relaunched=install\n$/);
    assert.ok(!existsSync(join(r.home, ".shroom/invite")));
  });

  it("an Apple silicon Mac whose Terminal runs under Rosetta still gets its build", async () => {
    const r = await run({ env: { FAKE_ARCH: "x86_64", FAKE_ARM64: "1" } });
    assert.equal(r.code, 0);
    assert.equal(r.installed, "1.2.0");
  });
});

describe("the download, counted (DECISIONS B228)", () => {
  const ZIP = `v${v120.version}/${v120.file}`;

  it("goes by shroom.am's counter, saying it's an install, and lands the same file", async () => {
    const r = await run({ env: { SHROOM_DOWNLOADS: `${base}/counted` } });
    assert.equal(r.code, 0, r.err);
    assert.equal(r.installed, "1.2.0");
    assert.deepEqual(counted, [`/counted/${ZIP}?via=install`], "once, and only the download");
  });

  it("carries our own runs' tag, and drops one that isn't a tag's shape", async () => {
    await run({ env: { SHROOM_DOWNLOADS: `${base}/counted`, SHROOM_REF: "ci" } });
    assert.deepEqual(counted, [`/counted/${ZIP}?via=install&ref=ci`]);
    const r = await run({ env: { SHROOM_DOWNLOADS: `${base}/counted`, SHROOM_REF: "a&via=update" } });
    assert.equal(r.installed, "1.2.0");
    assert.deepEqual(counted, [`/counted/${ZIP}?via=install`]);
  });

  it("a test's own releases are fetched where they are, uncounted", async () => {
    const r = await run({ env: { SHROOM_REF: "ci" } });
    assert.equal(r.installed, "1.2.0");
    assert.deepEqual(counted, []);
  });

  it("from the public releases, the counter is shroom.am's", () => {
    const script = readFileSync(SCRIPT, "utf8");
    assert.ok(script.includes('DOWNLOADS="${SHROOM_DOWNLOADS:-https://shroom.am/download}"'));
  });
});

describe("--early (DECISIONS B220)", () => {
  const pointer = (v) => `version=${v}\nmacos=14\narm64=${v120.file} ${sha(v120.bytes)}\n`;

  it("installs the early channel's build when it's newer than the latest release, and leaves the app on the channel", async () => {
    const r = await run({ args: ["--early"], with: { early: true, earlyManifest: pointer("1.3.0-early.1") } });
    assert.equal(r.code, 0);
    assert.deepEqual(r.out, ["Getting shroom…", "verified ✓", "Installed — opening shroom now."]);
    assert.equal(r.installed, "1.2.0");
    const file = join(r.home, ".shroom/channel");
    assert.equal(readFileSync(file, "utf8"), "early\n");
    assert.equal(statSync(file).mode & 0o777, 0o600);
    quiet(r);
  });

  it("with no early build out, installs the latest release, says so, and still leaves the app on the channel", async () => {
    const r = await run({ args: ["--early"] });
    assert.equal(r.code, 0);
    assert.deepEqual(r.out, [NO_EARLY, "Getting shroom…", "verified ✓", "Installed — opening shroom now."]);
    assert.equal(r.installed, "1.2.0");
    assert.equal(readFileSync(join(r.home, ".shroom/channel"), "utf8"), "early\n");
    quiet(r);
  });

  it("an early build the latest release has caught up with is passed over the same way", async () => {
    for (const v of ["1.2.0-early.3", "1.2.0", "1.1.9"]) {
      const r = await run({ args: ["--early"], with: { early: true, earlyManifest: pointer(v) } });
      assert.equal(r.code, 0, v);
      assert.equal(r.out[0], NO_EARLY, v);
      assert.equal(readFileSync(join(r.home, ".shroom/channel"), "utf8"), "early\n");
    }
  });

  it("updates an installed shroom onto the early channel, and still carries a join link", async () => {
    const r = await run({ args: ["--early", "--join", "https://v.myshroom.workers.dev/join/abc"], setup: runningOld, with: { early: true, earlyManifest: pointer("1.3.0-early.1") } });
    assert.equal(r.code, 0);
    assert.match(r.log, /open .*shroom\.app --args --relaunched=install-update --invite\n$/);
    assert.equal(readFileSync(join(r.home, ".shroom/channel"), "utf8"), "early\n");
  });

  it("the plain install line reads the latest release, says nothing of early builds and writes no channel", async () => {
    const r = await run({ with: { early: true, earlyManifest: pointer("1.3.0-early.1") } });
    assert.equal(r.code, 0);
    assert.deepEqual(r.out, ["Getting shroom…", "verified ✓", "Installed — opening shroom now."]);
    assert.ok(!existsSync(join(r.home, ".shroom/channel")));
  });
});

describe("updating", () => {
  it("quits the running app, swaps it, and reopens it quietly", async () => {
    const r = await run({ setup: runningOld });
    assert.equal(r.code, 0);
    assert.equal(r.out.at(-1), said("updating")[0]);
    assert.match(r.log, /^quit am\.shroom\nopen .*shroom\.app --args --relaunched=install-update\n$/);
    assert.equal(r.installed, "1.2.0");
    quiet(r);
  });

  it("carries a join link into the updated app", async () => {
    const r = await run({ args: ["--join", "https://v.myshroom.workers.dev/join/abc"], setup: runningOld });
    assert.match(r.log, /open .*shroom\.app --args --relaunched=install-update --invite\n$/);
  });

  it("asks the copy in ~/Applications by its own name, and reads its own folder", async () => {
    // A developer build (am.shroom.dev) keeps ~/.shroom-dev: its quit and its status file.
    const r = await run({ setup: (home, running) => { installOld(home, "am.shroom.dev"); writeFileSync(running, ""); } });
    assert.match(r.log, /^quit am\.shroom\.dev\n/);
    const take = await run({ setup: (home) => { installOld(home, "am.shroom.dev"); recordingNow(home, ".shroom-dev"); } });
    assert.deepEqual(take.out, said("install-refused-midtake"));
    assert.equal(take.installed, "1.1.0");
  });

  it("a copy whose name isn't of shroom's shape is asked to quit as the release, am.shroom", async () => {
    for (const odd of ['am.shroom" & x', "am.shroom/../x", "other.app"]) {
      const r = await run({ setup: (home, running) => { installOld(home, odd); writeFileSync(running, ""); } });
      assert.match(r.log, /^quit am\.shroom\n/, odd);
    }
  });

  // The app's own rule (runtime::Flavor::of): `….dev` is a developer build whatever comes
  // before it, and anything else is the release — the retired preview's name too (B148).
  it("only a developer build keeps ~/.shroom-dev; every other name is the release's ~/.shroom", async () => {
    for (const [id, dir] of [["am.shroom.preview", ".shroom"], ["am.shroom.preview.dev", ".shroom-dev"], ["xam.shroom", ".shroom"]]) {
      const take = await run({ setup: (home) => { installOld(home, id); recordingNow(home, dir); } });
      assert.deepEqual(take.out, said("install-refused-midtake"), id);
      assert.equal(take.installed, "1.1.0", id);
    }
    // And a take in another build's folder says nothing about this one: the update runs.
    for (const [id, other] of [["am.shroom.preview", ".shroom-preview"], ["am.shroom", ".shroom-dev"], ["am.shroom.dev", ".shroom"]]) {
      const r = await run({ setup: (home) => { installOld(home, id); recordingNow(home, other); } });
      assert.equal(r.code, 0, `${id} with a take in ${other}`);
      assert.equal(r.installed, "1.2.0", id);
    }
  });

  it("changes nothing when the app won't close", async () => {
    const r = await run({ env: { FAKE_STUCK: "1" }, setup: runningOld });
    assert.equal(r.code, 1);
    assert.deepEqual(r.out.slice(-2), [DIDNT_QUIT, UNCHANGED]);
    assert.equal(r.installed, "1.1.0");
  });

  it("an old copy that can't be moved aside: put back as it was, open again, said honestly", async () => {
    const r = await run({ env: { FAKE_MV_FAIL: "Applications/shroom.app" }, setup: runningOld });
    assert.equal(r.code, 1);
    assert.deepEqual(r.out.slice(-2), [NO_ROOM, UNCHANGED]);
    assert.match(r.log, /^quit am\.shroom\nopen .*shroom\.app --args --relaunched=install-update\n$/, "shroom was closed, so it opens again");
    assert.equal(r.installed, "1.1.0");
    quiet(r);
  });

  it("a new copy that can't be placed: the old one goes back and opens again", async () => {
    const r = await run({ env: { FAKE_MV_FAIL: "new/shroom.app" }, setup: runningOld });
    assert.equal(r.code, 1);
    assert.deepEqual(r.out.slice(-2), [NO_ROOM, UNCHANGED]);
    assert.match(r.log, /^quit am\.shroom\nopen .*--relaunched=install-update\n$/);
    assert.equal(r.installed, "1.1.0");
    quiet(r);
  });

  it("an app that wasn't open stays closed when the swap can't finish", async () => {
    const r = await run({ env: { FAKE_MV_FAIL: "new/shroom.app" }, setup: (home) => installOld(home) });
    assert.deepEqual(r.out.slice(-2), [NO_ROOM, UNCHANGED]);
    assert.equal(r.log, "");
    assert.equal(r.installed, "1.1.0");
  });

  it("if even the old copy can't go back, it never claims nothing changed", async () => {
    const r = await run({ env: { FAKE_MV_FAIL: "new/shroom.app old.app" }, setup: runningOld });
    assert.equal(r.code, 1);
    assert.equal(r.out.at(-1), NO_ROOM);
    assert.ok(!r.out.includes(UNCHANGED));
    assert.equal(r.log, "quit am.shroom\n");
    quiet(r);
  });
});

// Bagrat's re-run with the latest already open said “Updated shroom to 0.1.12” (CHECKPOINT
// §10, 10): the latest that checks out is said before anything is downloaded (B156).
describe("already the latest", () => {
  const [UPDATED, OPENING, REPAIRED] = said("updating");
  const JOIN = ["--join", "https://v.myshroom.workers.dev/join/abc"];

  it("the spec's lines, with this version in them", () => {
    assert.equal(UPDATED, "Updated shroom to 1.2.0 — opening it now.");
    assert.equal(OPENING, "shroom 1.2.0 is up to date — opening it now.");
    assert.equal(REPAIRED, "Repaired shroom 1.2.0 — opening it now.");
  });

  it("open already: says so, downloads nothing, and brings it forward — never asked to quit", async () => {
    const r = await run({ setup: runningLatest });
    assert.equal(r.code, 0);
    assert.deepEqual(r.out, [OPENING]);
    assert.equal(r.hits, 1, "the manifest alone");
    assert.match(r.log, /^open .*shroom\.app --args --relaunched=install-update\n$/, "no quit: nothing for macOS to ask about (B44)");
    assert.equal(r.installed, "1.2.0 (here before)");
    assert.deepEqual(r.leftovers, []);
    quiet(r);
  });

  it("not open: says so, downloads nothing, and opens it quietly", async () => {
    const r = await run({ setup: (home) => installLatest(home) });
    assert.equal(r.code, 0);
    assert.deepEqual(r.out, [OPENING]);
    assert.equal(r.hits, 1);
    assert.match(r.log, /^open .*shroom\.app --args --relaunched=install-update\n$/);
    assert.equal(r.installed, "1.2.0 (here before)");
    quiet(r);
  });

  it("with an invite, an open shroom is reopened to read it — still nothing downloaded", async () => {
    const r = await run({ args: JOIN, setup: runningLatest });
    assert.equal(r.code, 0);
    assert.deepEqual(r.out, [OPENING]);
    assert.equal(r.hits, 1);
    assert.match(r.log, /^quit am\.shroom\nopen .*shroom\.app --args --relaunched=install-update --invite\n$/);
    assert.equal(JSON.parse(readFileSync(join(r.home, ".shroom/invite"), "utf8")).link, JOIN[1]);
  });

  it("with an invite, a shroom that won't close changes nothing", async () => {
    const r = await run({ args: JOIN, env: { FAKE_STUCK: "1" }, setup: runningLatest });
    assert.equal(r.code, 1);
    assert.deepEqual(r.out, [DIDNT_QUIT, UNCHANGED]);
    assert.ok(!existsSync(join(r.home, ".shroom/invite")));
  });

  it("the latest that doesn't check out is downloaded again and swapped in, said as a repair", async () => {
    const r = await run({ env: { FAKE_CODESIGN_FAIL: "installed" }, setup: runningLatest });
    assert.equal(r.code, 0);
    assert.deepEqual(r.out, ["Getting shroom…", "verified ✓", REPAIRED]);
    assert.match(r.log, /^quit am\.shroom\nopen .*shroom\.app --args --relaunched=install-update\n$/);
    assert.equal(r.installed, "1.2.0", "the release's copy, fresh");
    quiet(r);
  });

  it("an older version is an update, as before", async () => {
    const r = await run({ setup: (home) => installLatest(home, "am.shroom", "1.1.0") });
    assert.equal(r.out.at(-1), UPDATED);
    assert.equal(r.installed, "1.2.0");
  });

  it("another build of the same version (a developer's) is replaced by the release's", async () => {
    const r = await run({ setup: (home) => installLatest(home, "am.shroom.dev") });
    assert.equal(r.out.at(-1), UPDATED);
    assert.equal(r.installed, "1.2.0");
  });

  it("mid-recording, the latest still stops first, changing nothing", async () => {
    const r = await run({ setup: (home) => { installLatest(home); recordingNow(home); } });
    assert.deepEqual(r.out, said("install-refused-midtake"));
    assert.equal(r.hits, 0);
  });
});

describe("refusals", () => {
  it("mid-recording: stops before fetching anything", async () => {
    const r = await run({ setup: (home) => { installOld(home); recordingNow(home); } });
    assert.equal(r.code, 1);
    assert.deepEqual(r.out, said("install-refused-midtake"));
    assert.equal(r.hits, 0);
    assert.equal(r.installed, "1.1.0");
  });

  it("a status file left by an app that isn't running says nothing", async () => {
    const r = await run({ setup: (home) => {
      mkdirSync(join(home, ".shroom"));
      writeFileSync(join(home, ".shroom/status.json"), JSON.stringify({ v: 1, pid: 999999, take_open: true }));
    } });
    assert.equal(r.code, 0);
    assert.equal(r.installed, "1.2.0");
  });

  // The seam's v2 shape (DECISIONS B80, ARCHITECTURE §11.2): v1's `pid` and `take_open`
  // where they were, beside what a guiding agent reads — pretty-printed, as the app writes it.
  const v2 = (takeOpen, take) => JSON.stringify({
    v: 2, pid: process.pid, app: "1.1.0", take_open: takeOpen, take, setup: "all-set",
    permission: { screen: "granted", mic: "granted", camera: "not-asked", chapter: "done" },
    join: { role: "member", invite: "none", invite_kind: "none", standing: "fine" },
  }, null, 2);

  it("mid-recording, as the app's v2 status says it: refused the same", async () => {
    const r = await run({ setup: (home) => { installOld(home); mkdirSync(join(home, ".shroom")); writeFileSync(join(home, ".shroom/status.json"), v2(true, "paused")); } });
    assert.equal(r.code, 1);
    assert.deepEqual(r.out, said("install-refused-midtake"));
    assert.equal(r.hits, 0);
    assert.equal(r.installed, "1.1.0");
  });

  it("a v2 status with no take open: the update runs", async () => {
    const r = await run({ setup: (home, running) => { runningOld(home, running); mkdirSync(join(home, ".shroom")); writeFileSync(join(home, ".shroom/status.json"), v2(false, "none")); } });
    assert.equal(r.code, 0, r.out.join("\n"));
    assert.equal(r.installed, "1.2.0");
    quiet(r);
  });

  it("an older macOS: stops before fetching anything", async () => {
    const r = await run({ env: { FAKE_MACOS: "13.6.1" } });
    assert.equal(r.code, 1);
    assert.deepEqual(r.out, ["shroom needs macOS 14 or newer — this Mac is on 13.6.1."]);
    assert.match(said("install-unsupported-os")[0], /^shroom needs macOS N or newer — this Mac is on M\.$/);
    assert.equal(r.hits, 0);
  });

  // A Linux box, a server, a coding agent's sandbox — shroom.am/agent hands agents the
  // install line, so it gets run off a Mac too. One line, then nothing else.
  it("a machine that isn't a Mac: one line, nothing fetched, nothing touched", async () => {
    const shapes = {
      "Linux, no sw_vers": { FAKE_OS: "Linux", FAKE_MACOS: "none" },
      "not Darwin, whatever sw_vers says": { FAKE_OS: "Linux" },
      "Darwin with no sw_vers": { FAKE_MACOS: "none" },
    };
    for (const [what, env] of Object.entries(shapes)) {
      const r = await run({ env });
      assert.equal(r.code, 1, what);
      assert.deepEqual(r.out, [NOT_A_MAC], what);
      assert.equal(r.err, "", `${what}: no shell's complaint`);
      assert.equal(r.hits, 0, what);
      assert.equal(r.log, "", `${what}: neither quit nor opened`);
      assert.deepEqual(readdirSync(r.home), ["log"], `${what}: nothing made in the home folder`);
    }
  });

  it("a release that needs a newer macOS than this Mac's", async () => {
    const r = await run({ env: { FAKE_MACOS: "14.6" }, with: { floor: 15 } });
    assert.equal(r.code, 1);
    assert.deepEqual(r.out, ["shroom needs macOS 15 or newer — this Mac is on 14.6."]);
    assert.equal(r.hits, 1, "nothing but the manifest was fetched");
  });

  it("a fingerprint that doesn't match is never installed — the spec's own line, for this check alone", async () => {
    const r = await run({ setup: (home) => installOld(home), with: { badSum: true } });
    assert.equal(r.code, 1);
    assert.deepEqual(r.out, ["Getting shroom…", CHECKSUM]);
    assert.equal(r.installed, "1.1.0");
    assert.deepEqual(r.leftovers, []);
  });

  it("a download whose signature doesn't hold is never installed, and says it was the signature (DECISIONS B125)", async () => {
    const r = await run({ env: { FAKE_CODESIGN_FAIL: "1" } });
    assert.equal(r.code, 1);
    assert.deepEqual(r.out, ["Getting shroom…", SIGNATURE]);
    quiet(r);
    assert.equal(r.installed, null);
    assert.deepEqual(r.leftovers, []);
  });

  it("from the public releases, only the release's own signer is installed — the pin is tools/release-identity.sha1", () => {
    const script = readFileSync(SCRIPT, "utf8");
    const pin = readFileSync(new URL("../../tools/release-identity.sha1", import.meta.url), "utf8").trim();
    assert.ok(script.includes(`\nRELEASE_LEAF=${pin}\n`), "the installer's pin is the release certificate's");
    const guarded = script.split('if [ "$RELEASES" = "https://github.com/bagrat/shroom/releases" ]; then')[1]?.split("\n  fi\n")[0] ?? "";
    assert.equal(script.split('if [ "$RELEASES" = "https://github.com/bagrat/shroom/releases" ]; then').length, 2, "one pin, for the download and the copy already here alike");
    assert.ok(guarded.includes('-R "=identifier \\"am.shroom\\" and certificate leaf = H\\"$RELEASE_LEAF\\""'), guarded);
    assert.ok(script.includes('RELEASES="${SHROOM_RELEASES:-https://github.com/bagrat/shroom/releases}"'), "the default it guards is the public one");
  });

  // The fixtures' codesign is a stub, and the pin runs only against the public releases —
  // so the pin's own line, as written, goes to the real codesign here, on a tiny app
  // signed ad hoc. It must be refused for not being the release's signer (exit 3), never
  // because codesign couldn't read what it was asked (0.1.9's `-R="=…"`: a syntax error).
  it("the pin's own line asks codesign in a form it reads, and is refused by the signer", () => {
    const script = readFileSync(SCRIPT, "utf8");
    const pin = readFileSync(new URL("../../tools/release-identity.sha1", import.meta.url), "utf8").trim();
    const signed = "signed() {" + script.split("\nsigned() {")[1].split("\n}\n")[0] + "\n}\n";
    const work = mkdtempSync(join(scratch, "pin-"));
    const a = join(work, "new/shroom.app");
    mkdirSync(join(a, "Contents/MacOS"), { recursive: true });
    execFileSync("clang", ["-arch", "arm64", "-x", "c", "-", "-o", join(a, "Contents/MacOS/shroom")], { input: "int main(void){return 0;}\n" });
    execFileSync("plutil", ["-create", "xml1", join(a, "Contents/Info.plist")]);
    execFileSync("plutil", ["-insert", "CFBundleIdentifier", "-string", "am.shroom", join(a, "Contents/Info.plist")]);
    execFileSync("plutil", ["-insert", "CFBundleExecutable", "-string", "shroom", join(a, "Contents/Info.plist")]);
    execFileSync("/usr/bin/codesign", ["-s", "-", "--force", a], { stdio: "ignore" });
    const r = spawnSync("sh", ["-c", `RELEASES=https://github.com/bagrat/shroom/releases RELEASE_LEAF=${pin} work='${work}'
codesign() { /usr/bin/codesign "$@" 2>>"$work/said"; echo "$?" >>"$work/rc"; }
${signed}
signed "$work/new/shroom.app" || :`], { encoding: "utf8" });
    assert.equal(r.status, 0, r.stderr);
    const said = readFileSync(join(work, "said"), "utf8");
    assert.equal(readFileSync(join(work, "rc"), "utf8"), "0\n3\n", `the signature holds whole, and its signer is refused: ${said}`);
    assert.doesNotMatch(said, /syntax/i);
    assert.match(said, /failed to satisfy/);
  });

  it("a dropped download installs nothing, says so without curl's words, and running it again works", async () => {
    const r = await run({ with: { drop: true } });
    assert.equal(r.code, 1);
    assert.equal(r.out.at(-1), NET_DROP);
    assert.equal(r.installed, null);
    quiet(r);
    assert.equal((await run()).installed, "1.2.0");
  });

  it("no connection at all", async () => {
    const r = await run({ env: { SHROOM_RELEASES: "http://127.0.0.1:9" } });
    assert.equal(r.code, 1);
    assert.deepEqual(r.out, [NET_DROP]);
    quiet(r);
    assert.equal(r.err, "");
  });

  it("a server that keeps failing: still the connection line, with no retry chatter", async () => {
    const r = await run({ with: { assetStatus: 503 } });
    assert.equal(r.code, 1);
    assert.equal(r.out.at(-1), NET_DROP);
    quiet(r);
  });

  it("a release whose file isn't there is ours to fix — never 'check your connection'", async () => {
    const asset = await run({ with: { assetStatus: 404 } });
    assert.equal(asset.code, 1);
    assert.equal(asset.out.at(-1), NOT_ALL_THERE);
    quiet(asset);
    const manifest = await run({ with: { manifestStatus: 404 } });
    assert.deepEqual(manifest.out, [NOT_ALL_THERE]);
    quiet(manifest);
  });

  it("a release whose file holds no shroom is ours to fix too", async () => {
    const r = await run({ with: { hollow: true } });
    assert.equal(r.code, 1);
    assert.deepEqual(r.out, ["Getting shroom…", NOT_ALL_THERE]);
    assert.equal(r.installed, null);
    quiet(r);
  });

  it("a kind of Mac the release has no build for", async () => {
    const r = await run({ with: { noArm: true } });
    assert.equal(r.code, 1);
    assert.deepEqual(r.out, [NO_BUILD]);
  });

  it("an Intel Mac (no Apple silicon key) is told it has no build, not handed one", async () => {
    const r = await run({ env: { FAKE_ARCH: "x86_64", FAKE_ARM64: "none" } });
    assert.equal(r.code, 1);
    assert.deepEqual(r.out, [NO_BUILD]);
    quiet(r);
  });
});

describe("this Mac can't take it", () => {
  it("a home folder with no room for the download folder", async () => {
    const r = await run({ setup: (home) => chmodSync(home, 0o555) });
    assert.equal(r.code, 1);
    assert.deepEqual(r.out, [NO_ROOM]);
    assert.equal(r.hits, 0);
    quiet(r);
  });

  it("no ~/Applications to place it in", async () => {
    const r = await run({ setup: (home) => writeFileSync(join(home, "Applications"), "") });
    assert.equal(r.code, 1);
    assert.equal(r.out.at(-1), NO_ROOM);
    assert.equal(r.log, "");
    quiet(r);
  });

  it("~/Applications that won't take it", async () => {
    const r = await run({ setup: (home) => { mkdirSync(join(home, "Applications")); chmodSync(join(home, "Applications"), 0o555); } });
    assert.equal(r.code, 1);
    assert.equal(r.out.at(-1), NO_ROOM);
    assert.equal(r.installed, null);
    assert.equal(r.log, "");
    quiet(r);
  });

  it("a verified download that can't be unpacked here (a full disk)", async () => {
    const r = await run({ env: { FAKE_DITTO_FAIL: "No space left on device" } });
    assert.equal(r.code, 1);
    assert.equal(r.out.at(-1), NO_ROOM);
    quiet(r);
  });
});

// A coding agent's sandbox can't write to ~/Applications, check a signature against its
// signer or open an app (Claude Code's and Codex's may say so in the environment); one
// that doesn't say so shows itself when a write is "Operation not permitted", or when
// even a system app's signature doesn't verify.
describe("somewhere restricted", () => {
  for (const [what, env] of Object.entries({ "Claude Code's sandbox": { SANDBOX_RUNTIME: "1" }, "Codex's sandbox": { CODEX_SANDBOX: "seatbelt" } })) {
    it(`${what}: one line, nothing fetched, nothing touched`, async () => {
      const r = await run({ env, setup: runningOld });
      assert.equal(r.code, 1);
      assert.deepEqual(r.out, [RESTRICTED]);
      assert.equal(r.hits, 0);
      assert.equal(r.log, "", "neither quit nor opened");
      assert.equal(r.installed, "1.1.0");
      assert.deepEqual(r.leftovers, []);
      quiet(r);
    });
  }

  it("a place it isn't permitted to write: said as somewhere restricted, not as no room", async () => {
    const r = await run({ env: { FAKE_MV_FAIL: "new/shroom.app", FAKE_MV_ERR: "Operation not permitted" } });
    assert.equal(r.code, 1);
    assert.equal(r.out.at(-1), RESTRICTED);
    assert.equal(r.installed, null);
    quiet(r);
  });

  it("an update it isn't permitted to swap in: the old copy goes back, and nothing changed", async () => {
    const r = await run({ env: { FAKE_MV_FAIL: "new/shroom.app", FAKE_MV_ERR: "Operation not permitted" }, setup: runningOld });
    assert.deepEqual(r.out.slice(-2), [RESTRICTED, UNCHANGED]);
    assert.equal(r.installed, "1.1.0");
    quiet(r);
  });

  it("a signature check that can't verify any app's, Calculator's too: said as somewhere restricted, nothing installed", async () => {
    const r = await run({ env: { FAKE_CODESIGN_FAIL: "all" }, setup: runningOld });
    assert.equal(r.code, 1);
    assert.deepEqual(r.out, ["Getting shroom…", RESTRICTED]);
    assert.equal(r.log, "", "neither quit nor opened");
    assert.equal(r.installed, "1.1.0");
    assert.deepEqual(r.leftovers, []);
    quiet(r);
  });

  it("an unpack it isn't permitted to write", async () => {
    const r = await run({ env: { FAKE_DITTO_FAIL: "Operation not permitted" } });
    assert.equal(r.out.at(-1), RESTRICTED);
    quiet(r);
  });
});

describe("macOS won't open it", () => {
  it("a fresh install stays placed and says where it is — never the system's error text", async () => {
    const r = await run({ env: { FAKE_OPEN_FAIL: "1" } });
    assert.equal(r.code, 1);
    assert.deepEqual(r.out, ["Getting shroom…", "verified ✓", said("installing")[1], DIDNT_OPEN]);
    assert.equal(r.installed, "1.2.0");
    assert.deepEqual(r.leftovers, []);
    quiet(r);
  });

  it("an update that swapped in but won't open says the same line", async () => {
    const r = await run({ env: { FAKE_OPEN_FAIL: "1" }, setup: runningOld });
    assert.equal(r.code, 1);
    assert.deepEqual(r.out.slice(-2), ["Updated shroom to 1.2.0 — opening it now.", DIDNT_OPEN]);
    assert.equal(r.installed, "1.2.0");
    assert.match(r.log, /--relaunched=install-update/);
    quiet(r);
  });

  it("an invite still waits in its file, for when shroom is opened by hand", async () => {
    const r = await run({ env: { FAKE_OPEN_FAIL: "1" }, args: ["--join", "https://v.myshroom.workers.dev/join/abc"] });
    assert.equal(r.out.at(-1), DIDNT_OPEN);
    assert.equal(JSON.parse(readFileSync(join(r.home, ".shroom/invite"), "utf8")).link, "https://v.myshroom.workers.dev/join/abc");
    quiet(r);
  });
});

describe("a manifest the app's updater wouldn't accept", () => {
  const good = sha(v120.bytes);
  const cases = {
    "a file name with a folder in it": `arm64=../${v120.file} ${good}`,
    "a file name with a slash": `arm64=a/${v120.file} ${good}`,
    "a hidden file name": `arm64=.${v120.file} ${good}`,
    "a short fingerprint": `arm64=${v120.file} ${good.slice(1)}`,
    "a fingerprint that isn't hex": `arm64=${v120.file} ${"g".repeat(64)}`,
    "a third part": `arm64=${v120.file} extra ${good}`,
    "an empty file name (a space before the fingerprint)": `arm64= ${good}`,
    "a fingerprint with no file name": `arm64=${good}`,
  };
  for (const [what, line] of Object.entries(cases)) {
    it(`${what}: nothing is fetched beyond the manifest, nothing installed`, async () => {
      const r = await run({ with: { manifest: `version=1.2.0\nmacos=14\n${line}\n` } });
      assert.equal(r.code, 1);
      assert.deepEqual(r.out, [NOT_ALL_THERE]);
      assert.equal(r.hits, 1);
      assert.equal(r.installed, null);
    });
  }
  for (const v of ["1.2.0/../x", "1..2", "1.2.0.4", "v1.2.0", ".1"]) {
    it(`a version shaped "${v}"`, async () => {
      const r = await run({ with: { manifest: `version=${v}\nmacos=14\narm64=${v120.file} ${good}\n` } });
      assert.deepEqual(r.out, [NOT_ALL_THERE]);
      assert.equal(r.hits, 1);
    });
  }
  it("an early build's version (B220) installs; any other suffix is refused", async () => {
    const early = (v) => `version=${v}\nmacos=14\narm64=${v120.file} ${sha(v120.bytes)}\n`;
    const ok = await run({ args: ["--early"], with: { early: true, earlyManifest: early("1.3.0-early.1") } });
    assert.equal(ok.code, 0);
    assert.equal(ok.installed, "1.2.0");
    for (const bad of ["1.3.0-beta.1", "1.3.0-early.0", "1.3.0-early.x", "1.3.0-early."]) {
      const r = await run({ args: ["--early"], with: { early: true, earlyManifest: early(bad) } });
      assert.deepEqual(r.out, [NOT_ALL_THERE], bad);
    }
  });
  it("a macOS floor that isn't a whole number", async () => {
    const r = await run({ with: { floor: "14.4" } });
    assert.deepEqual(r.out, [NOT_ALL_THERE]);
    assert.equal(r.hits, 1);
  });
  it("an upper-case fingerprint reads the same, as the updater reads it", async () => {
    const r = await run({ with: { manifest: `version=1.2.0\nmacos=14\narm64=${v120.file} ${good.toUpperCase()}\n` } });
    assert.equal(r.code, 0);
    assert.equal(r.installed, "1.2.0");
  });
});

describe("packaging a release (tools/package-release.sh)", () => {
  const PACK = join(ROOT, "tools/package-release.sh");
  // A real, tiny, ad-hoc-signed arm64 app — what the packager insists on — carrying its own
  // encoder and FFmpeg's notice and licence (B82): a release without them can't record.
  const app = join(scratch, "pack/shroom.app");
  const tinyApp = (a, { id = "am.shroom", encoder = "arm64", licence = true, floor } = {}) => {
    mkdirSync(join(a, "Contents/MacOS"), { recursive: true });
    execFileSync("clang", ["-arch", "arm64", "-x", "c", "-", "-o", join(a, "Contents/MacOS/shroom")], { input: "int main(void){return 0;}\n" });
    if (encoder) execFileSync("clang", ["-arch", encoder, "-x", "c", "-", "-o", join(a, "Contents/MacOS/ffmpeg")], { input: "int main(void){return 0;}\n" });
    if (licence) {
      mkdirSync(join(a, "Contents/Resources/licenses/ffmpeg"), { recursive: true });
      writeFileSync(join(a, "Contents/Resources/licenses/ffmpeg/NOTICE.txt"), "shroom includes FFmpeg\n");
      writeFileSync(join(a, "Contents/Resources/licenses/ffmpeg/COPYING.LGPLv2.1"), "GNU LESSER GENERAL PUBLIC LICENSE\n");
    }
    execFileSync("plutil", ["-create", "xml1", join(a, "Contents/Info.plist")]);
    execFileSync("plutil", ["-insert", "CFBundleIdentifier", "-string", id, join(a, "Contents/Info.plist")]);
    if (floor) execFileSync("plutil", ["-insert", "LSMinimumSystemVersion", "-string", floor, join(a, "Contents/Info.plist")]);
    execFileSync("codesign", ["-s", "-", "--force", "--deep", a], { stdio: "ignore" });
    return a;
  };
  before(() => tinyApp(app));
  const pack = (...args) => spawnSync("sh", [PACK, app, ...args], { encoding: "utf8" });

  it("writes a manifest the installer reads, with a whole-number floor", () => {
    const out = join(scratch, "pack/out");
    const r = pack("1.2.3", out, "15");
    assert.equal(r.status, 0, r.stdout + r.stderr);
    const m = readFileSync(join(out, "manifest.txt"), "utf8");
    assert.match(m, /^version=1\.2\.3\nmacos=15\narm64=shroom-1\.2\.3-arm64\.zip [0-9a-f]{64}\n$/);
  });

  for (const floor of ["14.4", "fourteen", "", "14 "]) {
    it(`refuses a macOS floor of ${JSON.stringify(floor)} — the updater reads whole numbers only`, () => {
      const out = join(scratch, `pack/bad-${floor.length}`);
      const r = pack("1.2.3", out, floor);
      assert.notEqual(r.status, 0);
      assert.ok(!existsSync(join(out, "manifest.txt")));
    });
  }

  for (const version of ["1..2", "1.2.3.4", ".1", "1.2."]) {
    it(`refuses a version shaped ${JSON.stringify(version)}`, () => {
      const r = pack(version, join(scratch, "pack/badv"));
      assert.notEqual(r.status, 0);
    });
  }
  // B62 (tests-sys #6): a dev build is never a release.
  const devApp = (name, id, marker) => {
    const a = join(scratch, `pack/${name}/shroom.app`);
    mkdirSync(join(a, "Contents/MacOS"), { recursive: true });
    const src = marker ? `static const char m[] __attribute__((used)) = "${marker}";\nint main(void){return m[0] == 0;}\n` : "int main(void){return 0;}\n";
    execFileSync("clang", ["-arch", "arm64", "-x", "c", "-", "-o", join(a, "Contents/MacOS/shroom")], { input: src });
    execFileSync("plutil", ["-create", "xml1", join(a, "Contents/Info.plist")]);
    execFileSync("plutil", ["-insert", "CFBundleIdentifier", "-string", id, join(a, "Contents/Info.plist")]);
    execFileSync("codesign", ["-s", "-", "--force", a], { stdio: "ignore" });
    return a;
  };
  const packApp = (a, out, env = {}) => spawnSync("sh", [PACK, a, "1.2.3", out], { encoding: "utf8", env: { ...process.env, ...env } });

  it("refuses a dev identifier", () => {
    const out = join(scratch, "pack/dev-out");
    const r = packApp(devApp("dev", "am.shroom.dev"), out);
    assert.notEqual(r.status, 0);
    assert.match(r.stdout, /am\.shroom\.dev is a dev build — never a release/);
    assert.ok(!existsSync(join(out, "manifest.txt")));
  });

  it("refuses a binary built with the test harness, whatever its identifier says", () => {
    const out = join(scratch, "pack/harness-out");
    const r = packApp(devApp("harness", "am.shroom", "shroom:dev-harness-build:never-a-release"), out);
    assert.notEqual(r.status, 0);
    assert.match(r.stdout, /carries the test harness — never a release/);
    assert.ok(!existsSync(join(out, "manifest.txt")));
  });

  it("packages a dev build only as a named stand-in, never with a publish command", () => {
    const out = join(scratch, "pack/standin-out");
    const r = packApp(devApp("standin", "am.shroom.dev", "shroom:dev-harness-build:never-a-release"), out, { SHROOM_STAND_IN: "1" });
    assert.equal(r.status, 0, r.stdout + r.stderr);
    assert.ok(existsSync(join(out, "manifest.txt")));
    assert.match(r.stdout, /stand-in release for tests \(am\.shroom\.dev, harness: yes\) — never publish it/);
    assert.doesNotMatch(r.stdout, /gh release create/);
  });

  it("the release build prints its publish command", () => {
    const r = pack("1.2.3", join(scratch, "pack/rel-out"));
    assert.equal(r.status, 0, r.stdout + r.stderr);
    assert.match(r.stdout, /gh release create v1\.2\.3/);
  });

  // Encode cluster #1 (B82): the release carries its encoder, arm64, with FFmpeg's licence.
  it("refuses a release that carries no encoder — it couldn't record", () => {
    const out = join(scratch, "pack/noenc-out");
    const r = packApp(tinyApp(join(scratch, "pack/noenc/shroom.app"), { encoder: null }), out);
    assert.notEqual(r.status, 0);
    assert.match(r.stdout, /carries no encoder — it can't record/);
    assert.ok(!existsSync(join(out, "manifest.txt")));
  });

  it("refuses a release whose encoder ships without FFmpeg's notice and licence", () => {
    const out = join(scratch, "pack/nolic-out");
    const r = packApp(tinyApp(join(scratch, "pack/nolic/shroom.app"), { licence: false }), out);
    assert.notEqual(r.status, 0);
    assert.match(r.stdout, /lacks FFmpeg's NOTICE\.txt/);
    assert.ok(!existsSync(join(out, "manifest.txt")));
  });

  it("refuses an encoder that isn't arm64 (B7)", () => {
    const out = join(scratch, "pack/x86enc-out");
    const r = packApp(tinyApp(join(scratch, "pack/x86enc/shroom.app"), { encoder: "x86_64" }), out);
    assert.notEqual(r.status, 0);
    assert.match(r.stdout, /expected an arm64 build, got: x86_64 in ffmpeg/);
  });

  // Encode cluster #2 (B6): the manifest's floor is the bundle's own unless told higher.
  it("takes the bundle's own floor, and refuses one below it", () => {
    const a = tinyApp(join(scratch, "pack/floor/shroom.app"), { floor: "15.0" });
    const out = join(scratch, "pack/floor-out");
    assert.equal(packApp(a, out).status, 0);
    assert.match(readFileSync(join(out, "manifest.txt"), "utf8"), /^macos=15$/m);
    const low = spawnSync("sh", [PACK, a, "1.2.3", join(scratch, "pack/floor-low"), "14"], { encoding: "utf8" });
    assert.notEqual(low.status, 0);
    assert.match(low.stdout, /below the app's own \(macOS 15\)/);
    const odd = packApp(tinyApp(join(scratch, "pack/floor-odd/shroom.app"), { floor: "14.4" }), join(scratch, "pack/floor-odd-out"));
    assert.notEqual(odd.status, 0, "a floor the updater can't read as a whole number");
  });
});

// The install line asked to uninstall (uninstall-sh, B135): for a Mac whose shroom won't open.
describe("uninstalling", () => {
  const UN = ["--uninstall"];
  // An installed shroom with everything it leaves: its folder (a token and a take that
  // reached the space), the system's caches and preferences, and a copy saved to Movies.
  const installed = (home, id = "am.shroom", dir = ".shroom") => {
    installOld(home, id);
    mkdirSync(join(home, dir, "takes/20261014-120000-0001"), { recursive: true });
    writeFileSync(join(home, dir, "token.json"), "{}");
    writeFileSync(join(home, dir, "takes/20261014-120000-0001/take.json"), "{}");
    writeFileSync(join(home, dir, "takes/20261014-120000-0001/uploaded"), "");
    for (const d of [`Library/WebKit/${id}`, `Library/Caches/${id}`, `Library/HTTPStorages/${id}`, `Library/Saved Application State/${id}.savedState`, "Library/Preferences", "Movies"]) mkdirSync(join(home, d), { recursive: true });
    writeFileSync(join(home, `Library/Preferences/${id}.plist`), "");
    writeFileSync(join(home, "Movies/shroom recording.mp4"), "");
  };
  const take = (home, name, files, dir = ".shroom") => {
    mkdirSync(join(home, dir, "takes", name), { recursive: true });
    for (const f of files) writeFileSync(join(home, dir, "takes", name, f), "{}");
  };
  const gone = (r, id = "am.shroom", dir = ".shroom") => {
    for (const p of ["Applications/shroom.app", dir, `Library/WebKit/${id}`, `Library/Caches/${id}`, `Library/HTTPStorages/${id}`, `Library/Saved Application State/${id}.savedState`, `Library/Preferences/${id}.plist`]) {
      assert.ok(!existsSync(join(r.home, p)), `${p} is gone`);
    }
  };

  it("takes back the shroom command: its link and its marked PATH line, nothing else", async () => {
    const MARK = "# shroom's command line (shroom.am) — run shroom --help";
    const r = await run({
      args: UN,
      setup: (home) => {
        installed(home);
        mkdirSync(join(home, ".local/bin"), { recursive: true });
        symlinkSync(join(home, "Applications/shroom.app/Contents/MacOS/shroom-cli"), join(home, ".local/bin/shroom"));
        writeFileSync(join(home, ".zprofile"), `export FOO=1\n\n${MARK}\nexport PATH="$PATH:$HOME/.local/bin"\nexport BAR=2\n`);
      },
    });
    assert.equal(r.code, 0);
    assert.throws(() => lstatSync(join(r.home, ".local/bin/shroom")), "the link itself is gone, not left dangling");
    assert.equal(readFileSync(join(r.home, ".zprofile"), "utf8"), "export FOO=1\n\nexport BAR=2\n");
  });

  // The security review, 2026-10-18: the person moved or deleted shroom's PATH line and kept
  // the mark — the line after the mark is theirs now, and stays.
  it("takes back only shroom's own PATH line: a line of theirs after the mark stays", async () => {
    const MARK = "# shroom's command line (shroom.am) — run shroom --help";
    const r = await run({
      args: UN,
      setup: (home) => {
        installed(home);
        writeFileSync(join(home, ".zprofile"), `export FOO=1\n${MARK}\neval "$(/opt/homebrew/bin/brew shellenv)"\numask 077\n`);
      },
    });
    assert.equal(r.code, 0);
    assert.equal(readFileSync(join(r.home, ".zprofile"), "utf8"), `export FOO=1\neval "$(/opt/homebrew/bin/brew shellenv)"\numask 077\n`);
  });

  it("removes the app, its folder and what the system keeps for it, and says so in the spec's words", async () => {
    const r = await run({ args: UN, setup: (home) => installed(home) });
    assert.equal(r.code, 0);
    assert.deepEqual(r.out, uninstall.slice(0, 2));
    // A member's Mac too, whose space was never theirs (CHECKPOINT §10, 11c; B156).
    assert.doesNotMatch(r.out[1], /your space/i);
    gone(r);
    assert.ok(existsSync(join(r.home, "Movies/shroom recording.mp4")), "a copy saved to Movies stays");
    assert.equal(r.hits, 0, "nothing is fetched: the space and the releases are never asked");
    quiet(r);
  });

  it("resets its permissions while the app is still there, then lets LaunchServices forget it", async () => {
    const r = await run({ args: UN, setup: (home) => installed(home) });
    const app = join(r.home, "Applications/shroom.app");
    assert.equal(r.log, [
      `lsregister -f ${app}`,
      "tccutil reset All am.shroom",
      `lsregister -u ${app}`,
      "defaults delete am.shroom",
    ].join("\n") + "\n", "registered first: tccutil finds an app only through LaunchServices (the probe, B135)");
  });

  it("quits a running shroom first, the way an update does", async () => {
    const r = await run({ args: UN, setup: (home, running) => { installed(home); writeFileSync(running, ""); } });
    assert.equal(r.code, 0);
    assert.match(r.log, /^quit am\.shroom\nlsregister -f /);
    gone(r);
  });

  it("a shroom that won't close: nothing is removed", async () => {
    const r = await run({ args: UN, env: { FAKE_STUCK: "1" }, setup: (home, running) => { installed(home); writeFileSync(running, ""); } });
    assert.equal(r.code, 1);
    assert.deepEqual(r.out, [DIDNT_QUIT, UNCHANGED]);
    assert.ok(existsSync(join(r.home, "Applications/shroom.app")) && existsSync(join(r.home, ".shroom/token.json")));
    assert.doesNotMatch(r.log, /tccutil/);
  });

  it("refuses mid-take, with the installer's own refusal, and changes nothing", async () => {
    const r = await run({ args: UN, setup: (home, running) => { installed(home); writeFileSync(running, ""); recordingNow(home); } });
    assert.equal(r.code, 1);
    assert.deepEqual(r.out, said("install-refused-midtake"));
    assert.equal(r.log, "", "neither quit nor reset");
    assert.ok(existsSync(join(r.home, "Applications/shroom.app")));
  });

  it("stops while recordings never reached the space, counting them in words", async () => {
    const two = await run({ args: UN, setup: (home) => {
      installed(home);
      take(home, "20261014-130000-0002", ["take.json", "finalized.json"]);
      take(home, "20261014-140000-0003", ["take.json", "finalized.json", "publish.json", "live"]);
      take(home, "20261014-150000-0004", ["take.json", "tombstoned"]); // deleted elsewhere: not this Mac's to keep
    } });
    assert.equal(two.code, 1);
    assert.deepEqual(two.out, [uninstall[3]]);
    assert.equal(two.log, "");
    assert.ok(existsSync(join(two.home, ".shroom/takes/20261014-130000-0002")) && existsSync(join(two.home, "Applications/shroom.app")));
    const one = await run({ args: UN, setup: (home) => { installed(home); take(home, "20261014-130000-0002", ["take.json"]); } });
    assert.deepEqual(one.out, [uninstall[4]]);
  });

  it("with no shroom here, says there's nothing to remove", async () => {
    const r = await run({ args: UN });
    assert.equal(r.code, 0);
    assert.deepEqual(r.out, [uninstall[2]]);
    assert.equal(r.log, "");
  });

  it("a folder left by an app already in the Trash is still removed", async () => {
    const r = await run({ args: UN, setup: (home) => { installed(home); rmSync(join(home, "Applications/shroom.app"), { recursive: true }); } });
    assert.equal(r.code, 0);
    assert.deepEqual(r.out, uninstall.slice(0, 2));
    gone(r);
  });

  it("a developer build is removed by its own name, with its own folder", async () => {
    const r = await run({ args: UN, setup: (home) => { installed(home, "am.shroom.dev", ".shroom-dev"); mkdirSync(join(home, ".shroom")); writeFileSync(join(home, ".shroom/token.json"), "{}"); } });
    assert.equal(r.code, 0);
    gone(r, "am.shroom.dev", ".shroom-dev");
    assert.match(r.log, /tccutil reset All am\.shroom\.dev/);
    assert.ok(existsSync(join(r.home, ".shroom/token.json")), "another build's folder is not this one's to remove");
  });

  // Only a name of shroom's own shape (am.shroom, or am.shroom.<part>…) is taken from the
  // copy in ~/Applications; any other is read as the release's, am.shroom.
  it("goes by shroom's own name: a copy naming itself anything else is removed as the release, and the system's shared folders stay", async () => {
    for (const odd of ["", "..", "am.shroom/../x", "am.shroom..x", 'am.shroom" & x', "other.app"]) {
      const r = await run({
        args: UN,
        setup: (home) => {
          installed(home);
          execFileSync("plutil", ["-replace", "CFBundleIdentifier", "-string", odd, join(home, "Applications/shroom.app/Contents/Info.plist")]);
          for (const d of ["Library/WebKit/someone.else", "Library/Caches/someone.else", "Library/HTTPStorages/someone.else"]) mkdirSync(join(home, d), { recursive: true });
        },
      });
      assert.equal(r.code, 0, JSON.stringify(odd));
      gone(r);
      for (const d of ["Library/WebKit/someone.else", "Library/Caches/someone.else", "Library/HTTPStorages/someone.else"]) {
        assert.ok(existsSync(join(r.home, d)), `${d} stays (${JSON.stringify(odd)})`);
      }
      assert.match(r.log, /^tccutil reset All am\.shroom$/m, JSON.stringify(odd));
      assert.match(r.log, /^defaults delete am\.shroom$/m, JSON.stringify(odd));
    }
  });

  it("refuses to run anywhere but a Mac, before touching anything", async () => {
    const r = await run({ args: UN, env: { FAKE_OS: "Linux" }, setup: (home) => installed(home) });
    assert.equal(r.code, 1);
    assert.deepEqual(r.out, [NOT_A_MAC]);
    assert.ok(existsSync(join(r.home, "Applications/shroom.app")));
  });
});

describe("the source view", () => {
  it("opens with the ratified header, verbatim", async () => {
    const header = machine.states["source-view"].said[0].text;
    assert.ok(readFileSync(SCRIPT, "utf8").startsWith(`#!/bin/sh\n${header}\n`), "install.sh starts, under its shebang, with the spec's header block");
  });

  it("is plain POSIX sh", async () => {
    assert.equal(spawnSync("sh", ["-n", SCRIPT]).status, 0);
  });
});
