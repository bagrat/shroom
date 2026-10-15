// Region 5's drives (library, settings, update) through the real app against the L1 fake,
// under a throwaway home. Run from app/: node test/region5-drive.mjs <library|settings>
// (the update drive goes through test/update-drive.sh, the uninstall drive through
// test/uninstall-drive.sh). Needs the debug bundle:
// npm run build:dev (in app/: the harness is built in only on purpose). Never touches the installed app.
// The Library and Settings drives also look at the keyboard (in front over another app, a
// typed field, ⌘W, ⌘Q — law 6, DEVIATIONS #38): that needs an unlocked screen. On a locked
// one the report lists what went unseen; SHROOM_HANDS=1 makes that a failure (the L5 lane,
// tests/l5/dock-vm.sh).
// A drive that passes leaves nothing in the temp folder; one that fails keeps its throwaway
// home — the run log is in it — and says where.
import { spawn } from "node:child_process";
import { existsSync, mkdtempSync, readFileSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { createInterface } from "node:readline";
import { refuseOffTestMachine } from "../../tests/test-machine.mjs";

refuseOffTestMachine("region5-drive.mjs");

const repo = new URL("../..", import.meta.url).pathname.replace(/\/$/, "");
// The uninstall drive runs a throwaway copy (test/uninstall-drive.sh names it): it removes itself.
const app = process.env.SHROOM_DRIVE_APP ?? join(repo, "target/debug/bundle/macos/shroom-dev.app");
const fakeBin = join(repo, "target/debug/cloudfake");
const which = process.argv[2];
const work = mkdtempSync(join(tmpdir(), `shroom-r5-${which}-`));
const fake = spawn(fakeBin, ["--state", "found"], { stdio: ["pipe", "pipe", "inherit"] });
const ep = await new Promise((ok, fail) => {
  createInterface({ input: fake.stdout }).once("line", (l) => ok(JSON.parse(l)));
  fake.once("exit", () => fail(new Error("fake exited")));
});
const out = join(work, "report.json");
const extra = process.argv.slice(3);
const run = spawn("open", ["-W", "-n", app, "--args", "--home", join(work, "home"), "--cloud-fake", ep.api, ep.r2, "--drive", which, out, ...extra], { stdio: "inherit" });
await new Promise((ok) => run.once("exit", ok));
if (which === "update") {
  // The swap reopens the throwaway copy; its second half writes beside the first.
  const second = `${out}.relaunched.json`;
  const until = Date.now() + 120000;
  while (!existsSync(second) && Date.now() < until) await new Promise((r) => setTimeout(r, 500));
  await new Promise((r) => setTimeout(r, 1500));
}
// The uninstall's helper removes the copy's home once it has quit (B135): give it ten seconds.
let homeLeft = false;
if (which === "uninstall") {
  const until = Date.now() + 10000;
  while (existsSync(join(work, "home")) && Date.now() < until) await new Promise((r) => setTimeout(r, 200));
  homeLeft = existsSync(join(work, "home"));
}
fake.stdin.end();
// A drive that wrote no report failed too — and its home is kept, like any failure's.
let reports;
try {
  reports = [out, `${out}.relaunched.json`].filter((f) => existsSync(f)).map((f) => JSON.parse(readFileSync(f, "utf8")));
  if (!existsSync(out)) throw new Error("the drive wrote no report");
} catch (e) {
  console.error(`${e.message}\nkept for a look: ${work}`);
  process.exit(1);
}
const [r, relaunched] = reports;
const unseen = reports.flatMap((x) => x.unseen ?? []);
const failed = reports.some((x) => x.ok === false) || homeLeft || (process.env.SHROOM_HANDS === "1" && unseen.length > 0);
if (homeLeft) console.error("the uninstalled copy's home is still there");
console.log(JSON.stringify({ ...(failed ? { work } : {}), report: r }, null, 1));
if (relaunched) console.log(JSON.stringify({ relaunched }, null, 1));
if (unseen.length) console.error(`not looked at (the screen is locked):\n- ${unseen.join("\n- ")}`);
if (failed) console.error(`kept for a look: ${work}`);
else rmSync(work, { recursive: true, force: true });
process.exit(failed ? 1 : 0);
