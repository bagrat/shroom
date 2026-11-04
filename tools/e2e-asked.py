#!/usr/bin/env python3
"""Did a push to a claude/… branch ask for an e2e run, or did it only bring main in?

    python3 tools/e2e-asked.py FILE BEFORE HEAD    # prints asked=true|false for $GITHUB_OUTPUT

e2e-full.yml and e2e-migration.yml start on a push that changes their request file
(dev/e2e/request, dev/e2e/migration-request). GitHub's `paths` filter reads the push's
whole diff, so a merge of main that carries another PR's request starts a run on this
branch too: a Mac's quarter hour spent on nobody's question. This asks the branch's own
commits instead: those the push added (BEFORE..HEAD) that main doesn't have. A plain
commit that touches FILE asks; a merge asks only when it leaves FILE unlike every parent
(the branch wrote new words while resolving it). Taking main's request, or keeping the
branch's old one, asks nothing.

BEFORE is the push's old head (`github.event.before`): all zeros for a new branch, and
possibly gone after a force push; either way the whole branch past main is read. Main is
origin/main, so the checkout needs it (fetch-depth: 0).
"""
import subprocess
import sys


def git(*args):
    return subprocess.run(["git", *args], capture_output=True, text=True)


def blob(rev, path):
    out = git("rev-parse", "--verify", "-q", f"{rev}:{path}")
    return out.stdout.strip() if out.returncode == 0 else None


def asked(path, before, head):
    span = [head, "^origin/main"]
    if before.strip("0") and git("cat-file", "-e", f"{before}^{{commit}}").returncode == 0:
        span.append(f"^{before}")
    out = git("rev-list", "--parents", *span)
    if out.returncode != 0:
        sys.exit(f"e2e-asked: can't list {' '.join(span)}: {out.stderr.strip()}")
    for line in out.stdout.splitlines():
        commit, *parents = line.split()
        mine = blob(commit, path)
        if all(blob(p, path) != mine for p in parents) and (parents or mine is not None):
            return True
    return False


def main(argv):
    if len(argv) != 3:
        sys.exit(__doc__)
    print(f"asked={'true' if asked(*argv) else 'false'}")


if __name__ == "__main__":
    main(sys.argv[1:])
