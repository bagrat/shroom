#!/usr/bin/env python3
"""Writes THIRD-PARTY-NOTICES.md: the licence of everything the macOS app ships that
isn't shroom's own (DECISIONS B123, B124).

What ships, and so what is listed:
  - every crate the app's binary links on the Mac (aarch64-apple-darwin): `shroom-app`'s
    normal dependencies, transitively, from `cargo metadata` — build and dev dependencies
    never reach the binary and are left out;
  - the npm packages the webviews' bundle carries (NPM_BUNDLED: what `vite build` puts in
    `app/dist`, read from its source maps), and hls.js, which the app uploads to every
    space and every viewer's browser runs (`worker/vendor/hls.min.js`).
FFmpeg, the bundled encoder, carries its own notice and licence (`licenses/ffmpeg/`).

For a dependency offered under a choice of licences (`A OR B`), the first of PREFERENCE
whose text the package ships is the one taken; `A AND B` takes both. A package that
publishes no licence file gets the standard text from `tools/licence-texts/` (or, for
MIT and BSD, the standard text with its authors as the copyright line); a git dependency
gets the files at its repository's root. Identical texts are printed once, with every
package under them.

  python3 tools/third-party-notices.py           write THIRD-PARTY-NOTICES.md
  python3 tools/third-party-notices.py --check   change nothing: fail if it isn't current

CI's Tests job and tools/release-build.sh run --check, so a dependency change without a
new notices file never ships. It needs cargo (which may fetch the Mac-only crates' sources
once) and the installed node packages.
"""
import json
import os
import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
OUT = ROOT / "THIRD-PARTY-NOTICES.md"
TEXTS = ROOT / "tools" / "licence-texts"
TARGET = "aarch64-apple-darwin"
NPM_BUNDLED = ["svelte", "@tauri-apps/api"]
PREFERENCE = ["MIT", "Apache-2.0", "ISC", "Zlib", "BSD-3-Clause", "BSD-2-Clause", "0BSD",
              "MIT-0", "Unlicense", "CC0-1.0", "Unicode-3.0", "BSL-1.0", "MPL-2.0", "CDLA-Permissive-2.0"]
LICENCE_FILE = re.compile(r"(?i)^(licen[cs]e|copying|notice|copyright|unlicense)")
# Files a package keeps but that don't grant the licence (an SPDX sidecar, a third party's list).
NOT_A_LICENCE = re.compile(r"(?i)\.(spdx|json|toml|rs|py|sh)$|third.?party")

MIT = """Copyright (c) {who}

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE."""

BSD3 = """Copyright (c) {who}

Redistribution and use in source and binary forms, with or without
modification, are permitted provided that the following conditions are met:

1. Redistributions of source code must retain the above copyright notice, this
   list of conditions and the following disclaimer.

2. Redistributions in binary form must reproduce the above copyright notice,
   this list of conditions and the following disclaimer in the documentation
   and/or other materials provided with the distribution.

3. Neither the name of the copyright holder nor the names of its
   contributors may be used to endorse or promote products derived from
   this software without specific prior written permission.

THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS "AS IS"
AND ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE
IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE ARE
DISCLAIMED. IN NO EVENT SHALL THE COPYRIGHT HOLDER OR CONTRIBUTORS BE LIABLE
FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL
DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR
SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS INTERRUPTION) HOWEVER
CAUSED AND ON ANY THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT LIABILITY,
OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE USE
OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE."""


def kind(text):
    """Which licence a file's text is, by its own words."""
    t = " ".join(text.split()).lower()
    if "mozilla public license" in t:  # before the AGPL: MPL-2.0 names it as a secondary licence
        return "MPL-2.0"
    if "gnu affero general public license" in t:
        return "AGPL"
    if "apache license" in t and "version 2.0" in t:
        return "Apache-2.0"
    if "unicode" in t and ("data files" in t or "unicode license" in t):
        return "Unicode-3.0"
    if "community data license agreement" in t:
        return "CDLA-Permissive-2.0"
    if "free and unencumbered software released into the public domain" in t:
        return "Unlicense"
    if "boost software license" in t:
        return "BSL-1.0"
    if "creative commons" in t and "cc0" in t:
        return "CC0-1.0"
    if "permission to use, copy, modify, and/or distribute" in t or "permission to use, copy, modify, and distribute" in t:
        return "0BSD" if "copyright notice and this permission notice appear" not in t else "ISC"
    if ("provided 'as-is'" in t or 'provided "as-is"' in t) and "altered source versions" in t:
        return "Zlib"
    if "permission is hereby granted" in t:
        return "MIT-0" if "the above copyright notice and this permission notice shall be included" not in t else "MIT"
    if "redistribution and use in source and binary forms" in t:
        return "BSD-3-Clause" if "neither the name" in t or "endorse or promote" in t else "BSD-2-Clause"
    return None


def alternatives(expr):
    """`(A OR B) AND C` as a list of conjunctions — each a set of licences, all owed."""
    # `X WITH exception` only adds permissions: X's text is what's owed.
    expr = re.sub(r"\s+WITH\s+\S+", "", expr.replace("/", " OR "))
    def parse(tokens):
        def atom():
            tok = tokens.pop(0)
            if tok == "(":
                v = disj()
                tokens.pop(0)
                return v
            return [[tok]]
        def conj():
            v = atom()
            while tokens and tokens[0] == "AND":
                tokens.pop(0)
                right = atom()
                v = [a + b for a in v for b in right]
            return v
        def disj():
            v = conj()
            while tokens and tokens[0] == "OR":
                tokens.pop(0)
                v = v + conj()
            return v
        return disj()
    return parse(re.findall(r"\(|\)|[^\s()]+", expr))


def files_in(d):
    out = {}
    for name in sorted(os.listdir(d)) if d.is_dir() else []:
        p = d / name
        if p.is_file() and LICENCE_FILE.match(name) and not NOT_A_LICENCE.search(name):
            text = p.read_text(encoding="utf-8", errors="replace").replace("\r\n", "\n").strip()
            out.setdefault(kind(text), []).append(text)
    return out


def standard(lic, who):
    if lic == "MIT":
        return MIT.format(who=who)
    if lic == "BSD-3-Clause":
        return BSD3.format(who=who)
    f = TEXTS / f"{lic}.txt"
    if f.exists():
        return f.read_text().strip()
    sys.exit(f"third-party-notices: no text for {lic} — add tools/licence-texts/{lic}.txt")


def choose(name, expr, found, who):
    """The licence texts owed for one package: the first satisfiable alternative it ships."""
    alts = sorted(alternatives(expr), key=lambda a: max(PREFERENCE.index(l) if l in PREFERENCE else 99 for l in a))
    for alt in alts:
        if all(l in found for l in alt):
            return alt, [t for l in alt for t in found[l]]
    alt = alts[0]
    for l in alt:
        if l not in PREFERENCE:
            sys.exit(f"third-party-notices: {name} is {expr}, and {l} isn't a licence this script knows — add it to PREFERENCE (and a text)")
    return alt, [found[l][0] if l in found else standard(l, who) for l in alt]


def git_root(manifest):
    """A git dependency's checkout: ~/.cargo/git/checkouts/<repo>/<rev>/."""
    d = Path(manifest).parent
    while d.parent.parent.name != "checkouts":
        if d == d.parent:
            return Path(manifest).parent
        d = d.parent
    return d


def crates():
    meta = json.loads(subprocess.run(
        ["cargo", "metadata", "--format-version", "1", "--locked", "--filter-platform", TARGET],
        cwd=ROOT, check=True, capture_output=True, text=True).stdout)
    pkgs = {p["id"]: p for p in meta["packages"]}
    nodes = {n["id"]: n for n in meta["resolve"]["nodes"]}
    ours = set(meta["workspace_members"])
    app = next(i for i in ours if pkgs[i]["name"] == "shroom-app")
    seen, todo = set(), [app]
    while todo:
        i = todo.pop()
        if i in seen:
            continue
        seen.add(i)
        todo += [d["pkg"] for d in nodes[i]["deps"] if any(k["kind"] is None for k in d["dep_kinds"])]
    out = []
    for i in seen:
        p = pkgs[i]
        if i in ours or p["source"] is None:  # shroom's own crates, and the empty workspace-hack stand-in
            continue
        d = Path(p["manifest_path"]).parent
        found = files_in(d)
        found.pop(None, None)
        if not found and p["source"].startswith("git+"):
            root = git_root(p["manifest_path"])
            for sub in (root, root / "licenses"):
                for k, v in files_in(sub).items():
                    if k:
                        found.setdefault(k, []).extend(v)
        who = ", ".join(re.sub(r"\s*<[^>]*>", "", a) for a in p.get("authors") or []) or f"the {p['name']} authors"
        src = p["source"].split("#")[0].split("?")[0].removeprefix("git+").removeprefix("registry+")
        where = p.get("repository") or (f"https://crates.io/crates/{p['name']}/{p['version']}" if "crates.io" in src else src)
        if p["source"].startswith("git+"):
            where = f"{src} (rev {p['source'].split('#')[1][:12]})"
        out.append(dict(name=p["name"], version=p["version"], expr=p["license"] or "", found=found, who=who, where=where))
    return out


def npm():
    out = []
    for name in NPM_BUNDLED + ["hls.js"]:
        d = ROOT / "node_modules" / name
        pj = json.loads((d / "package.json").read_text())
        found = files_in(d)
        found.pop(None, None)
        if name == "hls.js":
            # The package ships Apache-2.0's short notice (copyright + the boilerplate), not the
            # licence itself: the notice, then the licence, both owed.
            notice = (d / "LICENSE").read_text().strip()
            found = {"Apache-2.0": [notice + "\n\n" + standard("Apache-2.0", "")]}
        repo = pj.get("repository")
        repo = repo.get("url") if isinstance(repo, dict) else repo
        out.append(dict(name=name, version=pj["version"], expr=pj.get("license", ""), found=found,
                        who=pj.get("author", {}).get("name") if isinstance(pj.get("author"), dict) else (pj.get("author") or f"the {name} authors"),
                        where=(repo or f"https://www.npmjs.com/package/{name}").removeprefix("git+")))
    return out


# A copyright notice, not a line of the terms that happens to open with the word (MIT's
# "COPYRIGHT HOLDERS BE LIABLE…" when wrapped, "copyright notice and this permission…").
COPYRIGHT = re.compile(r"(?i)^\s*(portions\s+)?(copyright\b(?!\s+(holders?|notice|owner|license|and\b|or\b|law))|\(c\)|©)")


def split_copyright(text):
    """A licence text as (its copyright lines, the rest): the rest is what components share.
    A template's placeholder line (the Apache appendix's `Copyright [yyyy] …`) stays in."""
    holders, body = [], []
    for line in text.split("\n"):
        if COPYRIGHT.match(line) and "[yyyy]" not in line and "{yyyy}" not in line:
            holders.append(line.strip())
        else:
            body.append(line)
    return holders, "\n".join(body).strip()


TITLE = re.compile(r"(?i)^\s*(the\s+)?(mit|isc|zlib|bsd[\w -]*|apache|mozilla public|unicode[\w ,-]*)\s+licen[cs]e\b.*$")


def terms(body):
    """The licence's terms, as components share them: its title line and, for Apache-2.0,
    the "how to apply" appendix after END OF TERMS AND CONDITIONS (instructions to authors,
    not terms) left out; Apache-2.0's own header lines kept."""
    lines = body.split("\n")
    if kind(body) != "Apache-2.0":
        while lines and (not lines[0].strip() or TITLE.match(lines[0])):
            lines.pop(0)
    text = "\n".join(lines).strip()
    end = text.find("END OF TERMS AND CONDITIONS")
    if kind(text) == "Apache-2.0" and end != -1:
        text = text[: end + len("END OF TERMS AND CONDITIONS")]
    return text


def render():
    # Texts that differ only in their copyright lines are one licence: printed once, with
    # each component's own copyright lines listed above it (as every licence here asks:
    # its copyright notice and its terms, both reproduced).
    groups, rows = {}, []
    for kind_, items in (("crate", crates()), ("npm", npm())):
        for it in sorted(items, key=lambda x: (x["name"], x["version"])):
            alt, texts = choose(it["name"], it["expr"] or "MIT", it["found"], it["who"])
            rows.append((kind_, it, " AND ".join(alt)))
            for t in texts:
                holders, body = split_copyright(t)
                body = terms(body)
                key = re.sub(r"[^a-z0-9]", "", body.lower())
                g = groups.setdefault(key, {"body": body, "users": {}})
                g["users"].setdefault(f"{it['name']} {it['version']}", [])
                for h in holders:
                    if h not in g["users"][f"{it['name']} {it['version']}"]:
                        g["users"][f"{it['name']} {it['version']}"].append(h)
    lines = [
        "# Third-party notices",
        "",
        "shroom's macOS app is built with the open-source components below. Their licences",
        "are reproduced here as those licences require, and this file ships inside the app at",
        "`Contents/Resources/licenses/third-party/NOTICES.md`. The bundled FFmpeg encoder carries",
        "its own notice and licence in `Contents/Resources/licenses/ffmpeg/`.",
        "",
        "Where a component is offered under a choice of licences, shroom takes it under the one",
        "named in the table. Every component is used unmodified; its source is at the address",
        "given (for the MPL-2.0 components, that is where their Source Code Form is available).",
        "Each licence text below is printed once, after the copyright notices of every component",
        "that ships under it.",
        "",
        "*Generated by `tools/third-party-notices.py` — don't edit by hand.*",
        "",
        "## Components",
        "",
        "| Component | Version | Kind | Licence taken | Source |",
        "|---|---|---|---|---|",
    ]
    for kind_, it, taken in rows:
        lines.append(f"| {it['name']} | {it['version']} | {kind_} | {taken} | {it['where']} |")
    lines += ["", "## Licence texts", ""]
    ordered = sorted(groups.values(), key=lambda g: (kind(g["body"]) or "", -len(g["users"]), min(g["users"])))
    for n, g in enumerate(ordered, 1):
        lines += [f"### {n}. {kind(g['body']) or 'Licence'}", ""]
        for user in sorted(g["users"]):
            notices = g["users"][user]
            lines.append(f"- **{user}** — {'; '.join(notices)}" if notices else f"- **{user}**")
        lines += ["", "```", g["body"].replace("```", "\'\'\'"), "```", ""]
    return "\n".join(lines).rstrip() + "\n"


if __name__ == "__main__":
    text = render()
    if sys.argv[1:] == ["--check"]:
        if OUT.read_text() != text:
            sys.exit("THIRD-PARTY-NOTICES.md isn't current for what the app ships — run python3 tools/third-party-notices.py and commit it")
        print("third-party notices: current")
    else:
        OUT.write_text(text)
        print(f"third-party notices: {len(text) // 1024} KB written")
