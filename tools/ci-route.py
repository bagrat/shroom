#!/usr/bin/env python3
"""Which of CI's lanes a pull request needs: the Linux lanes always, the Mac's only when
the change can reach what runs on a Mac (ci.yml's `route` job).

    python3 tools/ci-route.py BASE HEAD    # prints mac=true|false (and why) for $GITHUB_OUTPUT
    python3 tools/ci-route.py --reads      # the web paths a Mac lane reads, and who reads them
    python3 tools/ci-route.py --self-test  # the router's own cases

A change skips the Mac only when every path it touches is web (WEB below) and no Mac lane
reads it. "Reads" is found, not listed: every path-shaped string in the Mac lanes' sources
(`include_str!`, `root.join("site/index.html")`, a node test's `join(ROOT, …)`), so a new
test that reads a doc routes that doc to the Mac the day it lands. Anything not web —
crates/, app/, tools/ (but a site-only one), installer/, .github/, spec/machine.json (but a
change to the landing's own state and the ledger, DEVIATIONS #58), a lockfile, a new
top-level folder — runs the full Mac set. When in doubt, the Mac runs.
"""
import json
import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent

# What a Mac lane can't be reached by unless it reads it: the site, the Worker, the docs,
# the spec's readable form, dev tooling, the frozen record, the session skills, the
# top-level prose, and the tools that make only the site (render-og-card.mjs draws
# site/og.png, and site/test/og.test.mjs holds it). spec/machine.json is not here: every
# region's tests derive from it.
WEB_DIRS = ("site/", "worker/", "docs/", "dev/", "history/", ".claude/")
WEB_FILES = ("spec/MACHINE.md", "tools/render-og-card.mjs")


def web(path):
    if path.startswith(WEB_DIRS) or path in WEB_FILES:
        return True
    return "/" not in path and path.endswith(".md")


# The Linux lane's crates, as .github/scripts/rust-tests.sh names them: their tests run on
# Linux on every pull request, so what they read needs no Mac.
def linux_crates():
    script = (ROOT / ".github/scripts/rust-tests.sh").read_text()
    m = re.search(r'^LINUX_CRATES="([^"]*)"', script, re.M)
    if not m:
        sys.exit("ci-route: .github/scripts/rust-tests.sh names no LINUX_CRATES")
    return m.group(1).split()


def mac_sources():
    """Every source a Mac lane runs: the Rust outside the Linux lane's crates, and the
    installer's node tests."""
    linux = set(linux_crates())
    roots = [ROOT / "app/src-tauri", ROOT / "tests/e2e-live"]
    roots += [d for d in sorted((ROOT / "crates").iterdir()) if d.is_dir() and d.name not in linux]
    for r in roots:
        for f in sorted(r.rglob("*.rs")):
            if "target" not in f.relative_to(ROOT).parts:
                yield f
    yield from sorted((ROOT / "installer/test").glob("*.mjs"))


# Path-shaped strings in those sources that read nothing: assertion text, not files.
NOT_READS = {
    "dev/",  # runtime/mod.rs: "nothing from the tree runs on his Mac" — a substring it forbids
}
LITERAL = re.compile(r'"((?:\.\./|/)*[A-Za-z0-9_.@-]+(?:/[A-Za-z0-9_.@*-]*)+)"')


def reads():
    """{web path a Mac lane reads: [the files that read it]}."""
    found = {}
    for src in mac_sources():
        text = src.read_text(errors="replace")
        for m in LITERAL.finditer(text):
            raw = m.group(1)
            line_start = text.rfind("\n", 0, m.start()) + 1
            near = text[line_start:m.start()]
            if "include_str!(" in near[-14:] or "include_bytes!(" in near[-16:]:
                rel = (src.parent / raw).resolve()
                try:
                    path = rel.relative_to(ROOT).as_posix()
                except ValueError:
                    continue
            else:
                path = re.sub(r"^(?:\.\./|/)+", "", raw)
            if path in NOT_READS or not web(path):
                continue
            found.setdefault(path, set()).add(src.relative_to(ROOT).as_posix())
    return found


# spec/machine.json is not web, but a change confined to the landing's own state and the
# ledger can't reach a Mac lane: the app doesn't build from the JSON, and the one Mac lane
# that reads it (the installer's tests) reads other states, by id (DEVIATIONS #58, ratified
# by Bagrat 2026-10-26). Edges, rules, laws, groups, flags and every other state still run
# the Mac. The self-test holds that no Mac lane names the landing.
SPEC = "spec/machine.json"
LANDING_STATES = {"landing"}
SPEC_WEB = {"ledger"} | {f"states.{s}" for s in LANDING_STATES}


def machine_diff():
    import importlib.util
    sys.dont_write_bytecode = True  # no tools/__pycache__ left in the tree
    spec = importlib.util.spec_from_file_location("machine_diff", ROOT / "tools/machine-diff.py")
    mod = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(mod)
    return mod


def landing_read_by_mac():
    """The landing's states a Mac lane that reads the JSON names, quoted."""
    named = set()
    for src in mac_sources():
        text = src.read_text(errors="replace")
        if SPEC in text:
            named |= {s for s in LANDING_STATES if f'"{s}"' in text}
    return named


def spec_reach(a, b, named):
    """Why a change to the JSON from a to b reaches the Mac, or None when it can't."""
    beyond = sorted(machine_diff().touched(a, b) - SPEC_WEB)
    if beyond:
        return f"touches {', '.join(beyond[:5])}{' …' if len(beyond) > 5 else ''}"
    if named:
        return f"the landing's state, which a Mac lane reads ({', '.join(sorted(named))})"
    return None


def spec_at(rev):
    out = subprocess.run(["git", "show", f"{rev}:{SPEC}"], cwd=ROOT, capture_output=True, text=True)
    return json.loads(out.stdout) if out.returncode == 0 else None


def reaches_mac(path, read):
    if not web(path):
        return "not web"
    for r in read:
        if path == r or path.startswith(r.rstrip("/") + "/"):
            return f"read by {', '.join(sorted(read[r]))}"
    return None


def reach(path, read, base, head):
    if path != SPEC:
        return reaches_mac(path, read)
    mb = subprocess.run(["git", "merge-base", base, head], cwd=ROOT, capture_output=True, text=True).stdout.strip()
    a, b = spec_at(mb) if mb else None, spec_at(head)
    if a is None or b is None:
        return "not web (added or removed)"
    r = spec_reach(a, b, landing_read_by_mac())
    return f"not web: {r}" if r else None


def route(base, head):
    out = subprocess.run(["git", "diff", "--name-only", "--no-renames", f"{base}...{head}"], cwd=ROOT, capture_output=True, text=True)
    if out.returncode != 0:
        print(f"::warning::ci-route: can't diff {base}...{head} ({out.stderr.strip()}) — the Mac runs")
        return True, ["(no diff)"]
    changed = [p for p in out.stdout.splitlines() if p]
    # A rename's old side counts as touched too: --no-renames lists both.
    read = reads()
    why = [f"{p} — {r}" for p in changed if (r := reach(p, read, base, head))]
    return bool(why) or not changed, why or (["(no changed files)"] if not changed else [])


def self_test():
    read = reads()
    cases = {
        "crates/shroom-core/src/lib.rs": True,
        "app/src/main.ts": True,
        "tools/release-build.sh": True,
        "installer/install.sh": True,
        ".github/workflows/ci.yml": True,
        "spec/machine.json": True,
        "package-lock.json": True,
        "THIRD-PARTY-NOTICES.md": True,  # the app bundles it (include_str!)
        "worker/src/worker.js": True,  # the app deploys it (include_str!)
        "tools/ci-route.py": True,  # the router itself
        "tools/render-machine-md.py": True,  # writes the spec's readable form: not site-only
        "site/report.js": True,
        "docs/RELEASING.md": True,
        "docs/CHECKPOINT.md": True,
        "docs/TESTING-PLAN.md": True,
        "worker/test/worker.test.js": True,  # walked for `tart run`
        "docs/BUILD-STATUS.md": False,
        "docs/DECISIONS.md": False,  # read by shroom-core's tests, which run on Linux
        "spec/MACHINE.md": False,
        "dev/AGENT-RUNS.md": False,
        "history/README.md": False,
        "README.md": False,
        "site/agent.md": False,
        "site/index.html": False,  # the checkpoint's landing half runs on Linux (landing.test.mjs)
        "tools/render-og-card.mjs": False,  # draws site/og.png and nothing else
        "worker/package.json": False,
    }
    bad = [f"{p}: want mac={w}, got {reaches_mac(p, read)!r}" for p, w in cases.items() if bool(reaches_mac(p, read)) != w]

    # A change to spec/machine.json skips the Mac only when it touches the landing's own
    # state and the ledger, and no Mac lane names the landing (DEVIATIONS #58, ratified).
    import copy
    spec = json.loads((ROOT / "spec/machine.json").read_text())

    def changed(edit):
        b = copy.deepcopy(spec)
        edit(b)
        return b

    def ledger_row(b):
        b["ledger"].append({"id": "N999", "title": "a landing change", "decision": "x", "status": "ratified"})

    def landing(b):
        b["states"]["landing"]["summary"] += " More."
        b["states"]["landing"].setdefault("cites", []).append("N999")

    spec_cases = {
        "the landing's state and a ledger row": (lambda b: (landing(b), ledger_row(b)), set(), False),
        "a ledger row alone": (ledger_row, set(), False),
        "nothing": (lambda b: None, set(), False),
        "the landing, but a Mac lane names it": (landing, {"landing"}, True),
        "source-view (the installer's test reads it)": (lambda b: b["states"]["source-view"].update(summary="x"), set(), True),
        "a new state": (lambda b: b["states"].update({"landing-two": dict(spec["states"]["landing"])}), set(), True),
        "an edge from the landing": (lambda b: b["edges"].append({**spec["edges"][0], "label": "x"}), set(), True),
        "a law": (lambda b: b["laws"].append("x"), set(), True),
        "a rule": (lambda b: b["rules"][0].update(status="provisional"), set(), True),
    }
    for name, (edit, named, want) in spec_cases.items():
        got = spec_reach(spec, changed(edit), named)
        if bool(got) != want:
            bad.append(f"spec/machine.json, {name}: want mac={want}, got {got!r}")
    landing_named = landing_read_by_mac()
    if landing_named:
        bad.append(f"a Mac lane reads the landing's state ({', '.join(sorted(landing_named))}): DEVIATIONS #58 assumes none does")
    if bad:
        sys.exit("ci-route self-test:\n  " + "\n  ".join(bad))
    print(f"ci-route self-test: {len(cases) + len(spec_cases)} cases hold; the Mac lanes read {len(read)} web paths")


def main(argv):
    if argv == ["--self-test"]:
        return self_test()
    if argv == ["--reads"]:
        for p, by in sorted(reads().items()):
            print(f"{p}  ←  {', '.join(sorted(by))}")
        return
    if len(argv) != 2:
        sys.exit(__doc__)
    mac, why = route(*argv)
    print(f"mac={'true' if mac else 'false'}")
    for w in why:
        print(f"  {w}", file=sys.stderr)


if __name__ == "__main__":
    main(sys.argv[1:])
