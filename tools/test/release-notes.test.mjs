import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { problems, body, notesPath, UNDER_THE_HOOD } from '../release-notes.mjs';

test('a plain list in product voice passes', () => {
  assert.deepEqual(problems('- Closing the lid pauses your recording.\n- Stop is now Finish & Share.\n'), []);
});

test('notes with no list are refused', () => {
  assert.match(problems('Lots of fixes.\n').join('\n'), /no list/);
});

test("the build's insides are refused, by line", () => {
  for (const line of ['- Lid close pauses (B92)', '- See #79', '- CLAUDE.md: no separate go', '- Four CI flakes', '- Worker routes', '- by @bagrat', '- Machine-json: every action names a state in machine.json', '- Several PRs merged']) {
    assert.notDeepEqual(problems(line), [], line);
  }
});

test('the body is what\'s new, then the generated list folded under the hood', () => {
  const generated = "## What's Changed\n* CLAUDE.md: a rule by @bagrat in https://github.com/bagrat/shroom/pull/54\n\n\n**Full Changelog**: https://github.com/bagrat/shroom/compare/v0.1.10...v0.1.12";
  const out = body('- Stop is now Finish & Share.\n', generated);
  assert.ok(out.startsWith("## What's new\n\n- Stop is now Finish & Share.\n"));
  assert.ok(out.includes(`<details>\n<summary>${UNDER_THE_HOOD}</summary>\n\n* CLAUDE.md`));
  assert.ok(!out.includes("What's Changed"));
  assert.ok(out.trimEnd().endsWith('</details>'));
  // A blank line after <summary> and before </details>: GitHub renders the markdown inside.
  assert.match(out, /<\/summary>\n\n/);
  assert.match(out, /\n\n<\/details>/);
});

test('with nothing generated, there is no fold', () => {
  assert.ok(!body('- One thing.\n', "## What's Changed\n").includes('<details>'));
});

test("0.1.12's notes are readable", () => {
  assert.deepEqual(problems(readFileSync(notesPath('0.1.12'), 'utf8')), []);
});
