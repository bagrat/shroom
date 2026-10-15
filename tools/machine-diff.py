#!/usr/bin/env python3
"""What a change to spec/machine.json changes, in words: the change list a PR that touches
the machine carries (B133, machine-json phase 5), for Bagrat to read instead of the JSON.

    python3 tools/machine-diff.py [base] [head]   # default: origin/main, the working tree
    python3 tools/machine-diff.py --regions ...   # also: the regions it touches, one a line

base and head are git revisions; "-" (or no head) is the working tree. Prints Markdown:
states, said-lines, actions, edges, rules and groups added, removed or changed, each with
its status and citations, and what changed status (provisional → ratified, and back).
"""
import json
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
PATH = "spec/machine.json"


def load(rev):
    if rev in (None, "-"):
        return json.loads((ROOT / PATH).read_text())
    out = subprocess.run(["git", "show", f"{rev}:{PATH}"], cwd=ROOT, capture_output=True, text=True)
    if out.returncode != 0:
        sys.exit(f"machine-diff: no {PATH} at {rev}: {out.stderr.strip()}")
    return json.loads(out.stdout)


def cites(cs):
    return f" ({', '.join(cs)})" if cs else ""


def quote(t):
    return "“" + t.replace("\n", " ⏎ ") + "”"


def diff(a, b):
    out, regions = [], set()
    w = out.append
    sa, sb = a["states"], b["states"]

    def region(sid):
        s = sb.get(sid) or sa.get(sid)
        regions.add(s["region"])
        return s["region"]

    # States.
    for sid in sorted(set(sb) - set(sa), key=lambda x: (sb[x]["region"], x)):
        s = sb[sid]
        region(sid)
        w(f"- **New state `{sid}`** (Region {s['region']}, {s['kind']}, {s['status']}){cites(s.get('cites', []))}: {s['summary']}")
        for l in s["said"]:
            w(f"  - says {quote(l['text'])} ({l['copy']})")
    for sid in sorted(set(sa) - set(sb), key=lambda x: (sa[x]["region"], x)):
        region(sid)
        w(f"- **State gone: `{sid}`** (Region {sa[sid]['region']}, was {sa[sid]['status']})")
    for sid in sorted(set(sa) & set(sb), key=lambda x: (sb[x]["region"], x)):
        x, y = sa[sid], sb[sid]
        lines = []
        for key in ("status", "kind", "surface", "parent", "board", "region"):
            if x.get(key) != y.get(key):
                lines.append(f"{key}: {x.get(key)} → **{y.get(key)}**")
        for key, label in (("summary", "summary"), ("entry", "entry")):
            if x[key] != y[key]:
                lines.append(f"{label} rewritten: {y[key]}")
        if x.get("cites") != y.get("cites"):
            lines.append(f"cites: {', '.join(x.get('cites', []))} → **{', '.join(y.get('cites', []))}**")
        xl, yl = {l["id"]: l for l in x["said"]}, {l["id"]: l for l in y["said"]}
        for lid in yl.keys() - xl.keys():
            l = yl[lid]
            lines.append(f"says {quote(l['text'])} — new ({l['copy']}){cites(l.get('cites', []))}")
        for lid in xl.keys() - yl.keys():
            lines.append(f"no longer says {quote(xl[lid]['text'])}")
        for lid in xl.keys() & yl.keys():
            p, q = xl[lid], yl[lid]
            if p["text"] != q["text"]:
                lines.append(f"{quote(p['text'])} → **{quote(q['text'])}** ({q['copy']}){cites(q.get('cites', []))}")
            elif p["copy"] != q["copy"]:
                lines.append(f"{quote(q['text'])}: {p['copy']} → **{q['copy']}**{cites(q.get('cites', []))}")
        xa, ya = {a["label"]: a for a in x["actions"]}, {a["label"]: a for a in y["actions"]}
        for k in ya.keys() - xa.keys():
            lines.append(f"new action {k}" + (f" → `{ya[k]['goes_to']}`" if ya[k].get("goes_to") else ""))
        for k in xa.keys() - ya.keys():
            lines.append(f"action gone: {k}")
        for k in xa.keys() & ya.keys():
            # An empty value and no value mean the same: no target, no note.
            val = lambda act, f: act.get(f) if act.get(f) not in ("", None) else None
            changed = [f"{f}: {val(xa[k], f) or 'nothing'} → {val(ya[k], f) or 'nothing'}" for f in ("kind", "return_default", "goes_to", "note") if val(xa[k], f) != val(ya[k], f)]
            if changed:
                lines.append(f"action {k}: " + "; ".join(changed))
        if lines:
            region(sid)
            w(f"- **`{sid}`** (Region {y['region']}):")
            for l in lines:
                w(f"  - {l}")

    # Edges.
    ea = {(e["from"], e["to"]): e for e in a["edges"]}
    eb = {(e["from"], e["to"]): e for e in b["edges"]}
    for k in sorted(eb.keys() - ea.keys()):
        e = eb[k]
        region(k[0]); region(k[1])
        w(f"- **New edge** `{k[0]}` → `{k[1]}`: {e['label'] or e['trigger']} ({e['kind']}, {e['status']}){cites(e['cites'])}")
    for k in sorted(ea.keys() - eb.keys()):
        region(k[0]); region(k[1])
        w(f"- **Edge gone** `{k[0]}` → `{k[1]}`: {ea[k]['label'] or ea[k]['trigger']} (was {ea[k]['status']})")
    for k in sorted(ea.keys() & eb.keys()):
        p, q = ea[k], eb[k]
        changed = [f"{f}: {p[f]} → **{q[f]}**" for f in ("kind", "label", "trigger", "status") if p[f] != q[f]]
        if p["cites"] != q["cites"]:
            changed.append(f"cites: {', '.join(p['cites'])} → **{', '.join(q['cites'])}**")
        if changed:
            region(k[0]); region(k[1])
            w(f"- Edge `{k[0]}` → `{k[1]}`: " + "; ".join(changed))

    # Rules and groups.
    ra, rb = {r["id"]: r for r in a["rules"]}, {r["id"]: r for r in b["rules"]}
    for k in sorted(rb.keys() - ra.keys()):
        r = rb[k]
        w(f"- **New rule `{k}`** — {r['label']} ({r['status']}){cites(r['cites'])}: {', '.join(r['from'])} → {', '.join(r['to'])} on {', '.join(r['on'])}. {r['says']}")
    for k in sorted(ra.keys() - rb.keys()):
        w(f"- **Rule gone: `{k}`** — {ra[k]['label']}")
    for k in sorted(ra.keys() & rb.keys()):
        p, q = ra[k], rb[k]
        changed = [f"{f}: {p.get(f)} → **{q.get(f)}**" for f in ("label", "from", "to", "on", "reverse", "family", "kind", "says", "status", "cites") if p.get(f) != q.get(f)]
        if changed:
            w(f"- Rule `{k}`: " + "; ".join(changed))
    ga, gb = {g["id"]: g for g in a["groups"]}, {g["id"]: g for g in b["groups"]}
    for k in sorted(gb.keys() | ga.keys()):
        p, q = ga.get(k), gb.get(k)
        if p is None:
            w(f"- **New group @{k}**: {', '.join(q['states'])}")
        elif q is None:
            w(f"- **Group gone: @{k}**")
        elif set(p["states"]) != set(q["states"]):
            add, drop = sorted(set(q["states"]) - set(p["states"])), sorted(set(p["states"]) - set(q["states"]))
            w(f"- Group @{k}: " + "; ".join(([f"adds {', '.join(add)}"] if add else []) + ([f"drops {', '.join(drop)}"] if drop else [])))

    # The rest: laws, the ledger.
    for key, label in (("laws", "the laws"), ("ledger", "the ledger"), ("absent_from_v1", "absent from v1"), ("scaffold_flags", "the scaffold flags")):
        if a.get(key) != b.get(key):
            w(f"- **{label}** changed")
    return out, sorted(regions)


def main():
    args = [x for x in sys.argv[1:] if not x.startswith("--")]
    base = args[0] if args else "origin/main"
    head = args[1] if len(args) > 1 else "-"
    lines, regions = diff(load(base), load(head))
    print(f"### What this changes in the machine ({base} → {'the working tree' if head == '-' else head})")
    print()
    print("\n".join(lines) if lines else "Nothing: spec/machine.json is unchanged.")
    if "--regions" in sys.argv:
        print()
        print("Regions touched: " + (", ".join(str(r) for r in regions) or "none"))


if __name__ == "__main__":
    main()
