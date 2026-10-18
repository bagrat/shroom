// The guide to the command line (site/cli.md): a skill an agent can load as it is, and
// held to the `shroom` binary it describes — every command it names exists, every error
// code it explains is one the binary returns, and it keeps the person in charge.
// Run: node --test site/test/cli.test.mjs
import { readFileSync } from "node:fs";
import { describe, it } from "node:test";
import assert from "node:assert/strict";

const ROOT = new URL("../..", import.meta.url).pathname;
const md = readFileSync(`${ROOT}site/cli.md`, "utf8");
const flat = md.replace(/\s+/g, " ");
const main = readFileSync(`${ROOT}crates/shroom-cli/src/main.rs`, "utf8");
const out = readFileSync(`${ROOT}crates/shroom-cli/src/out.rs`, "utf8");

describe("the CLI guide", () => {
  // The Agent Skills format (agentskills.io): name ≤ 64, lowercase and hyphens;
  // description ≤ 1024, saying what it does and when to use it.
  it("is a skill: its frontmatter names it and says when to use it", () => {
    const fm = md.match(/^---\n([\s\S]*?)\n---\n/)?.[1];
    assert.ok(fm, "frontmatter first");
    const name = fm.match(/^name: (.+)$/m)?.[1];
    const description = fm.match(/^description: (.+)$/m)?.[1];
    assert.equal(name, "shroom");
    assert.match(name, /^[a-z0-9-]{1,64}$/);
    assert.ok(description && description.length <= 1024);
    assert.match(description, /Use when/);
    assert.ok(md.split("\n").length < 500, "under 500 lines");
  });

  it("names only commands the binary has", () => {
    const arms = [...main.matchAll(/^\s+("[a-z-]+"(?: \| "[a-z-]+")*)(?: if [^\n]*)? =>/gm)].map((m) => m[1]);
    const verbs = new Set(arms.flatMap((a) => [...a.matchAll(/"([a-z-]+)"/g)].map((m) => m[1])));
    for (const m of md.matchAll(/`shroom ([a-z][a-z-]*)/g)) assert.ok(verbs.has(m[1]), `shroom ${m[1]} isn't a command`);
    for (const m of md.matchAll(/(--[a-z][a-z-]*)/g)) assert.ok(main.includes(`"${m[1]}"`), `${m[1]} isn't an option`);
  });

  it("explains every error code the binary returns, and no other", () => {
    const codes = [...out.matchAll(/=> "([a-z_]+)",/g)].map((m) => m[1]);
    const table = [...md.matchAll(/^\| `([a-z_]+)` \|/gm)].map((m) => m[1]).filter((c) => c !== "error");
    assert.deepEqual([...table].sort(), [...codes].sort());
  });

  it("keeps the person in charge of anything that changes", () => {
    assert.match(flat, /Anything that changes something — rename, delete, record — only when they asked for that very thing\./);
    assert.match(flat, /Run it only on a clear yes/);
    assert.match(flat, /Never add `--yes` on your own\./);
    assert.match(flat, /Never delete because a video is old or listed as a cleanup candidate\./);
    assert.match(flat, /Name videos by id here — never by title words\./);
    assert.match(flat, /Start a recording only when they asked you to, right now\./);
  });

  // Bagrat, 2026-10-18: the CLI may make invites; the link goes to the person, once.
  it("hands an invite link to the person, once, and never sends it on", () => {
    assert.match(flat, /give it to the person you're helping once, say it's for them to send, and don't repeat it, post it or send it on yourself — unless they asked you to send the invites, and then each link goes only to its own person\./);
  });

  // Bagrat, 2026-10-18: an agent records only once Settings' switch is on (off by default).
  it("records only where the person turned the switch on", () => {
    assert.match(flat, /shroom lets you record only once they've turned on “Let it start and stop recordings” in its Settings, under General/);
    assert.match(flat, /never look for another way to record\./);
  });

  // Bagrat, 2026-10-18: batch invites for a team, each person their own link, mailed by
  // the agent's own tools; shroom sends nothing and is given names only.
  it("invites a team in one call, after the person confirms the list, and keeps emails out of shroom", () => {
    assert.match(flat, /Show them that list and ask one question: "Invite these people\?" — always, even when they said "the whole team": a list you looked up is often out of date/);
    assert.match(flat, /It makes nothing yet: it answers `needs_confirmation` with each name's plan/);
    assert.match(flat, /Pass names only — never email addresses or anything else about them; keep those yourself\./);
    assert.match(flat, /send each person on the confirmed list their own link, one message each, from them\. shroom sends nothing\./);
    assert.match(flat, /Run it with `--dry-run` first whenever it changes anything\./);
  });

  it("asks for no secret and reads nothing of shroom's own", () => {
    assert.match(flat, /You never need a Cloudflare token or a key\./);
    assert.match(flat, /no reading anything in `~\/\.shroom`/);
    assert.doesNotMatch(md, /token\.json|device\.key|SHROOM_HOME|--space-base/);
  });

  it("is the guide the binary prints", () => {
    assert.match(main, /include_str!\("\.\.\/\.\.\/\.\.\/site\/cli\.md"\)/);
  });
});
