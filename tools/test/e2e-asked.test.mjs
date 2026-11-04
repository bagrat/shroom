// tools/e2e-asked.py: did a push to a claude/… branch ask for an e2e run, or did it only
// bring main's own requests in? Each case is a throwaway repo shaped like the real one: a
// `main` (as origin/main) and a branch, pushed from BEFORE to HEAD.
// Run: node --test tools/test/e2e-asked.test.mjs
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { execFileSync } from 'node:child_process';
import { mkdtempSync, writeFileSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';

const TOOL = new URL('../e2e-asked.py', import.meta.url).pathname;
const REQ = 'dev/e2e/request';
const ZERO = '0'.repeat(40);

function repo() {
  const dir = mkdtempSync(join(tmpdir(), 'e2e-asked-'));
  const git = (...a) => execFileSync('git', a, { cwd: dir, encoding: 'utf8', env: { ...process.env, GIT_AUTHOR_NAME: 't', GIT_AUTHOR_EMAIL: 't@example.com', GIT_COMMITTER_NAME: 't', GIT_COMMITTER_EMAIL: 't@example.com' } }).trim();
  const put = (path, text, msg) => {
    execFileSync('mkdir', ['-p', join(dir, path, '..')]);
    writeFileSync(join(dir, path), text);
    git('add', path);
    git('commit', '-q', '-m', msg);
    return git('rev-parse', 'HEAD');
  };
  git('init', '-q', '-b', 'main');
  put(REQ, 'asked=0\n', 'start');
  put('README.md', 'x\n', 'readme');
  git('update-ref', 'refs/remotes/origin/main', 'main');
  git('checkout', '-q', '-b', 'claude/x');
  const asked = (before, head = 'HEAD', file = REQ) =>
    execFileSync('python3', [TOOL, file, before, head], { cwd: dir, encoding: 'utf8' }).trim();
  // main moves on: another PR's request lands there.
  const mainAsks = (n) => {
    git('checkout', '-q', 'main');
    put(REQ, `asked=main-${n}\n`, `main asks ${n}`);
    git('update-ref', 'refs/remotes/origin/main', 'main');
    git('checkout', '-q', 'claude/x');
  };
  const merge = (...extra) => git('merge', '-q', '--no-edit', ...extra, 'main');
  return { dir, git, put, asked, mainAsks, merge, done: () => rmSync(dir, { recursive: true, force: true }) };
}

test("the branch's own request asks", () => {
  const r = repo();
  const before = r.git('rev-parse', 'HEAD');
  r.put(REQ, 'asked=1\n', 'e2e: mine');
  assert.equal(r.asked(before), 'asked=true');
  r.done();
});

test('merging main, which carries another PR\'s request, does not ask', () => {
  const r = repo();
  r.put('app.txt', 'work\n', 'work');
  const before = r.git('rev-parse', 'HEAD');
  r.mainAsks(1);
  r.merge();
  assert.equal(r.asked(before), 'asked=false');
  r.done();
});

test("a push that brings main in beside the branch's own request still asks", () => {
  const r = repo();
  const before = r.git('rev-parse', 'HEAD');
  r.put(REQ, 'asked=1\n', 'e2e: mine');
  r.mainAsks(1);
  try { r.merge(); } catch { /* both changed the line */ }
  // The conflict kept the branch's request.
  r.put(REQ, 'asked=1\n', 'Merge main');
  assert.equal(r.asked(before), 'asked=true');
  r.done();
});

test('a merge that resolves the request to new words asks: the branch wrote them', () => {
  const r = repo();
  r.put(REQ, 'asked=1\n', 'e2e: mine');
  const before = r.git('rev-parse', 'HEAD');
  r.mainAsks(1);
  try { r.merge(); } catch { /* conflict */ }
  r.put(REQ, 'asked=2\n', 'Merge main, asking again');
  assert.equal(r.asked(before), 'asked=true');
  r.done();
});

test("a merge that takes main's request, or keeps the branch's old one, does not ask", () => {
  for (const side of ['theirs', 'ours']) {
    const r = repo();
    r.put(REQ, 'asked=1\n', 'e2e: mine');
    const before = r.git('rev-parse', 'HEAD');
    r.mainAsks(1);
    r.merge('-X', side);
    assert.equal(r.asked(before), 'asked=false', side);
    r.done();
  }
});

test("a new branch's first push asks only if its own commits touch the request", () => {
  const r = repo();
  r.put('app.txt', 'work\n', 'work');
  assert.equal(r.asked(ZERO), 'asked=false');
  r.put(REQ, 'asked=1\n', 'e2e: mine');
  assert.equal(r.asked(ZERO), 'asked=true');
  r.done();
});

test('a push whose old head is gone (a force push) reads the whole branch', () => {
  const r = repo();
  r.put(REQ, 'asked=1\n', 'e2e: mine');
  assert.equal(r.asked('1234567890123456789012345678901234567890'), 'asked=true');
  r.done();
});

test('the other request file is read the same way', () => {
  const r = repo();
  const before = r.git('rev-parse', 'HEAD');
  r.put('dev/e2e/migration-request', 'asked=1\n', 'e2e: migration');
  assert.equal(r.asked(before, 'HEAD', 'dev/e2e/migration-request'), 'asked=true');
  assert.equal(r.asked(before), 'asked=false');
  r.done();
});
