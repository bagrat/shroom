// The page for coding agents (site/agent.md) says what the landing says, speaks to the
// agent that reads it, and asks before it runs anything.
// Run: node --test site/test/agent.test.mjs
import { readFileSync } from "node:fs";
import { describe, it } from "node:test";
import assert from "node:assert/strict";

const ROOT = new URL("../..", import.meta.url).pathname;
const md = readFileSync(`${ROOT}site/agent.md`, "utf8");
const flat = md.replace(/\s+/g, " ");
const html = readFileSync(`${ROOT}site/index.html`, "utf8");
const installer = readFileSync(`${ROOT}installer/install.sh`, "utf8");
const landing = JSON.parse(readFileSync(`${ROOT}spec/machine.json`, "utf8")).states.landing;

// The landing's ratified lines that belong to the page itself, not to a browser: no Copy
// button, no install tabs or "Read it first" link, no demo, no links at its foot (N55, N59), none
// of its words to the person about their agent, and not the price bar's short labels for a phone.
// The system-requirements sentence is checked against the landing's own (filled-in) sentence instead.
const PAGE_ONLY = /^(Copy|Send feedback|Send feedback · .*|Install in one line|Terminal|Homebrew|Coding agent|Read it first|Your agent asks you .*|Recorded with shroom\..*|\$0 \/ 3h video|\$1\.50 \/ 30h)$/;

/** Every “…” on the page: a line the agent says to the person, or a word it quotes. */
const said = [...flat.matchAll(/“([^”]+)”/g)].map((m) => m[1]);

describe("the agent page", () => {
  it("mirrors the landing's ratified lines", () => {
    for (const l of landing.said) {
      // The Homebrew road (N52) is the person's: the agent runs the install line it names.
      if (PAGE_ONLY.test(l.text) || /^For macOS/.test(l.text) || (l.cites ?? []).includes("copy table") || (l.cites ?? []).includes("N52")) continue;
      assert.ok(flat.includes(l.text), `missing: ${l.text}`);
    }
  });

  it("states the same system requirements as the landing", () => {
    const sysreq = html.match(/For macOS \d+ or newer, [^.<]*\./)?.[0];
    assert.ok(sysreq, "the landing states its system requirements");
    assert.ok(flat.includes(sysreq), `the agent page should say: ${sysreq}`);
  });

  it("names the landing's install command, exactly", () => {
    const cmd = html.match(/<code id="cmd">([^<]+)<\/code>/)?.[1];
    assert.equal(cmd, "curl -fsSL https://shroom.am/install | sh");
    assert.ok(md.includes(cmd));
  });

  // Its reader is the agent. "Your agent" is how the landing and the browser's frame speak
  // to the person; inside the page it would turn the reader into the wrong one.
  it("speaks to the agent that reads it", () => {
    assert.doesNotMatch(md, /your (coding )?agent/i);
    assert.match(md, /^## If you're an agent$/m);
    assert.match(flat, /Below is its home page, written for people: there, “you” means the person you're helping and “we” means shroom\./);
  });

  // The same page answers shroom.am/ to an agent that was only asked what shroom is.
  it("installs only when the person asked for it", () => {
    assert.match(flat, /If the person you're helping didn't ask you to install shroom, the page above is the answer: offer nothing, run nothing\./);
    assert.match(flat, /If they did, install it only with their yes: show them the command, ask one question, and run it only on their answer\./);
    assert.doesNotMatch(flat, /wants shroom on their Mac/);
  });

  it("asks before it runs anything, and keeps the token out of the chat", () => {
    assert.match(flat, /only with their yes: show them the command, ask one question, and run it only on their answer/);
    assert.match(flat, /On “Not now”, run nothing\. Silence, “maybe”, a question or an earlier “don't ask” is not a yes/);
    assert.match(flat, /On “Install shroom”, run the command once\./);
    assert.match(flat, /never into this chat/);
    assert.match(flat, /If one lands here anyway, don't use or repeat it; tell them to delete it in Cloudflare and make a fresh one\./);
  });

  // The command checks the Mac itself, and rightly (`uname -m` is wrong under Rosetta);
  // a check by the agent would be a command run before the ask.
  it("leaves the Mac to the command, and runs nothing else", () => {
    assert.match(flat, /Don't check their Mac first: the command does\./);
    assert.doesNotMatch(md, /sw_vers|uname/);
    assert.equal(md.match(/```/g)?.length, 2, "one fenced block: the command");
  });

  it("hands the command over when it can't run it on their Mac", () => {
    assert.match(flat, /If you can't run it that way \(a sandbox, a cloud, container or remote session, or a chat with no terminal\) or would rather not, don't try a sandboxed run: in one short sentence, have them paste it into Terminal on their Mac instead\./);
  });

  // In a sandbox the installer's own check fails on a good release (0.1.9's agent test).
  it("runs it outside any sandbox, or hands it over", () => {
    assert.match(flat, /Run it as a normal command, with network access and outside any sandbox/);
    // The person's own yes is the way through a tool's check; never a way around it.
    assert.match(flat, /Their answer to your question is the only approval there is: if your tool asks them to allow the run, that's theirs to click; if it refuses the run, don't try it another way — hand them the line for Terminal, in one sentence\./);
  });

  // One question the person answers with a click, not a block of text to read through.
  it("asks one multiple-choice question, and keeps the rest to itself", () => {
    assert.match(flat, /Ask one question: “Install shroom from shroom.am on this Mac\?” If you have a tool for asking the person a multiple-choice question, use it, with three choices: “Install shroom”, “I have an invite link” and “Not now”\. Otherwise ask it in one short line with the same three\./);
    assert.match(flat, /no summary of the script, no list of its steps, unless they ask/);
    for (const choice of ["Install shroom", "I have an invite link", "Not now"]) {
      assert.ok(choice.length <= 25, `a choice is a button's worth: ${choice}`);
    }
  });

  // The invite goes on the end the way the installer reads it: `--join <link>`, first.
  it("asks for the invite link and passes it as the installer expects", () => {
    assert.match(flat, /On “I have an invite link”, ask “Paste your invite link here\.” and, when it comes, run the command once with the link on the end, unchanged:/);
    assert.ok(md.includes("`curl -fsSL https://shroom.am/install | sh -s -- --join <their invite link>`"));
    assert.match(installer, /if \[ "\$\{1:-\}" = "--join" \]; then join="\$\{2:-\}"; fi/);
  });

  it("doesn't work around a stop, and asks again before a second run", () => {
    assert.match(flat, /If it stops, don't fix or work around it; a second run takes a new yes; if it says to run it in Terminal, give them the line and don't run it again here\./);
    assert.match(flat, /Keep what you add to a line: no list of causes or options\./);
  });

  // The two words the page waits for are the installer's own last lines.
  it("knows it's done by the installer's last line", () => {
    assert.match(flat, /When its last line says Installed, your part is done: shroom's window takes it from there\./);
    assert.match(flat, /If it says Updated, shroom is back as the mushroom in the menu bar\. Anything else means it didn't finish\./);
    assert.match(installer, /^ +say "Installed — opening shroom now\."$/m);
    assert.match(installer, /^ +say "Updated shroom to \$version — opening it now\."$/m);
  });

  // 125 characters: a fetch tool that summarises the page can't pass a longer quote on whole.
  it("gives the agent short lines to say, in shroom's words", () => {
    assert.ok(said.includes("Install shroom from shroom.am on this Mac?"), "the ask is a line to say");
    assert.ok(said.includes("Paste your invite link here."), "an invite has its own ask");
    for (const line of said) {
      assert.ok(line.length <= 125, `too long to survive a summary (${line.length}): ${line}`);
      assert.doesNotMatch(line, /\b(curl|sudo|checksum|SHA|manifest|Worker|bucket|R2|JSON|TCC)\b|~\//i, line);
    }
  });

  it("leaves trouble and questions to the person", () => {
    assert.match(flat, /theirs to send, not yours to file/);
  });

  it("is where the landing's agent line sends the agent", () => {
    const line = html.match(/<code id="agent">([^<]+)<\/code>/)?.[1];
    assert.equal(line, "Install shroom on my Mac: read https://shroom.am/agent and follow it.");
    assert.match(html, /<a href="\/agent">Read it first<\/a> <span>Your agent asks you before it runs anything\.<\/span>/);
  });
});

describe("the agent page's last line", () => {
  it("says visits are counted, as the landing's footer does", () => {
    assert.equal(md.trimEnd().split("\n").at(-1), "We count visits to this page — no cookies, and nothing that tells us who you are.");
  });
});
