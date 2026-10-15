#!/usr/bin/env python3
"""Assemble spec/machine.json (B133, machine-json phase 1) — a one-off.

Merges the old sources — spec/r1–r6.json (states, said-lines, actions, transitions),
spec/map.json (the drawn edges), spec/meta.json (laws, ledger, notation…) — under one id
per state (ids.py), then overlays the implementation wherever it differs: the states only
the code has, the copy that shipped under a logged deviation, and the moves the core makes
(moves.json, recorded by record-moves.patch over every shroom-core test).

    python3 dev/proposals/machine-json/assemble.py            # writes spec/machine.json
                                                              # and machine-report.md here
Retired with phase 3: spec/machine.json is the spec now, edited directly in PRs, and the
old sources are frozen in history/spec/. Re-running this would overwrite every edit since,
so it refuses unless given --rerun-phase-1 (and then writes the phase-1 JSON again).
"""
import collections
import json
import re
import sys

if "--rerun-phase-1" not in sys.argv:
    sys.exit("assemble.py is retired: spec/machine.json is edited directly now (see its docstring)")
from pathlib import Path

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[2]
sys.dont_write_bytecode = True  # no __pycache__ beside the tools
sys.path.insert(0, str(HERE))
from ids import MAP, MERGE, RENAME, canon  # noqa: E402
import overlay  # noqa: E402
import rules as R  # noqa: E402

load = lambda p: json.loads((ROOT / p).read_text())
FX = {i: load(f"history/spec/r{i}.json") for i in range(1, 7)}
MAPJ = load("history/spec/map.json")
META = load("history/spec/meta.json")
MOVES = json.loads((HERE / "moves.json").read_text())

report = collections.defaultdict(list)  # section -> lines


def note(section, line):
    report[section].append(line)


# ── states ───────────────────────────────────────────────────────────────────────────

def slug(text, n=4):
    words = re.findall(r"[a-z0-9]+", text.lower().replace("’", "'").replace("'", ""))
    return "-".join(words[:n]) or "line"


def said_ids(sid, lines):
    seen = collections.Counter()
    for l in lines:
        base = f"{sid}.{slug(l['text'])}"
        seen[base] += 1
        l["id"] = base if seen[base] == 1 else f"{base}-{seen[base]}"
    return lines


def ref(x):
    """A cite as the JSON keeps it: a ledger row (N24), a deviation (#44), a decision (B133)."""
    return x


states = {}
order = []
merged_away = {}
for (sid, r), (into, ri) in MERGE.items():
    merged_away[(sid, r)] = (into, ri)

for i in range(1, 7):
    for s in FX[i]["states"]:
        if (s["id"], i) in merged_away:
            continue
        cid = canon(s["id"])
        parent = canon(s["parent"]) if s.get("parent") else None
        st = {
            "region": i,
            "kind": s["kind"],
            "parent": parent,
            "surface": s["surface"],
            "status": s["status"],
            "board": None,
            "summary": s["summary"],
            "entry": s["entry_conditions"],
            "said": [{"text": l["text"], "role": l["role"], "copy": l["copy_status"], **({"note": l["note"]} if l.get("note") else {})} for l in s["said_lines"]],
            "actions": [{k: v for k, v in a.items() if v is not None and v is not False or k == "return_default"} for a in s["actions"]],
            "cites": list(s.get("decided_by") or []),
            "was": {"fixture": f"r{i}.json", "id": s["id"], "lines": s["source_lines"]},
        }
        for a in st["actions"]:
            if "goes_to" in a:
                a["goes_to"] = canon(a["goes_to"]) if canon(a["goes_to"]) in RENAME.values() or a["goes_to"] in RENAME else a["goes_to"]
        if cid in states:
            raise SystemExit(f"two states are {cid}")
        states[cid] = st
        order.append(cid)
        if cid != s["id"]:
            note("ids", f"`{s['id']}` → `{cid}`")

# The merged pairs: fold the second's summary, entry, lines and cites into the first.
for (sid, r), (into, ri) in MERGE.items():
    src = next(s for s in FX[r]["states"] if s["id"] == sid)
    dst = states[canon(into)]
    dst["summary"] += " " + src["summary"]
    if src["entry_conditions"] and src["entry_conditions"] not in dst["entry"]:
        dst["entry"] += " " + src["entry_conditions"]
    for l in src["said_lines"]:
        if l["text"] not in [x["text"] for x in dst["said"]]:
            dst["said"].append({"text": l["text"], "role": l["role"], "copy": l["copy_status"]})
    dst["cites"] = sorted(set(dst["cites"]) | set(src.get("decided_by") or []))
    dst["was"] = [dst["was"], {"fixture": f"r{r}.json", "id": sid, "lines": src["source_lines"]}]
    note("merged", f"`{sid}` (r{r}) and `{into}` (r{ri}) are one state, `{canon(into)}`: the two fixtures wrote it twice")

# Fixture parents written as prose or a retired name.
for cid, st in states.items():
    if st["parent"] and st["parent"] not in states:
        raise SystemExit(f"{cid}: parent {st['parent']} isn't a state")

overlay.apply(states, order, note, said_ids)

for cid in order:
    said_ids(cid, states[cid]["said"])

# ── edges ────────────────────────────────────────────────────────────────────────────

edges = {}  # (from, to) -> edge


def ends(name, out=False):
    if name in states:
        return [name]
    if out and name == "portchapter":
        return ["perm-priming"]
    m = MAP.get(name)
    if m is None:
        return None
    return m if isinstance(m, list) else [m]


def add_edge(a, b, **kw):
    e = edges.setdefault((a, b), {"from": a, "to": b, "kind": None, "label": "", "trigger": "", "status": None, "cites": [], "source": []})
    for k in ("kind", "label", "trigger"):
        if kw.get(k) and not e[k]:
            e[k] = kw[k]
    for c in kw.get("cites", []):
        if c not in e["cites"]:
            e["cites"].append(c)
    if kw.get("source") and kw["source"] not in e["source"]:
        e["source"].append(kw["source"])
    if kw.get("status") == "provisional" or e["status"] is None:
        e["status"] = kw.get("status") or e["status"]
    return e


notes = {n["name"]: n for n in MAPJ["notes"]}
for e in MAPJ["edges"]:
    if e["from"] in notes or e["to"] in notes:
        note("map-notes", f"map edge `{e['from']}` → `{e['to']}` ({e['label']}) touches a note, kept as the note's `about`")
        continue
    fs, ts = ends(e["from"], out=True), ends(e["to"])
    if fs is None or ts is None:
        raise SystemExit(f"map edge {e} has an end with no state")
    for f in fs:
        for t in ts:
            if f == t:
                continue
            add_edge(f, t, kind=e["kind"], label=e["label"], source="map")

RESOLVE = overlay.TRANSITION_TARGETS
for i in range(1, 7):
    for s in FX[i]["states"]:
        f = canon(s["id"])
        if (s["id"], i) in merged_away:
            f = canon(merged_away[(s["id"], i)][0])
        for tr in s["transitions"]:
            to = tr["to"]
            if canon(to) in states:
                ts = [canon(to)]
            elif to in RESOLVE:
                ts = RESOLVE[to]
            else:
                raise SystemExit(f"r{i} {s['id']} → {to!r}: no state")
            for t in ts:
                if t is None or t == f:
                    continue
                add_edge(f, t, trigger=tr["trigger"], source="fixture")

# The core's named edges (Arm.edge).
apply_rs = (ROOT / "crates/shroom-core/src/apply.rs").read_text()
arm_edges = set(re.findall(r'\("([^"]+)", "([^"]+)"\)', apply_rs))
for a, b in sorted(arm_edges):
    fs, ts = ends(canon(a), out=True), ends(canon(b))
    if fs is None or ts is None:
        if (a in notes or b in notes) and (fs or ts):
            note("map-notes", f"the core names `{a}` → `{b}`: a note, not a state (no move)")
        continue  # other string pairs in apply.rs
    for f in fs:
        for t in ts:
            if f != t:
                add_edge(f, t, source="core-arm")

# ── the moves the core made, against the edges and the rules ──────────────────────────

rules = R.RULES
groups = R.GROUPS
groups["banners"]["states"] = [k for k in order if states[k]["kind"] == "banner" or k in ("update-ready", "updating-app")]


def expand(spec):
    out = []
    for x in spec:
        if x.startswith("@"):
            out += groups[x[1:]]["states"]
        else:
            out.append(x)
    return out


def ancestors(s):
    out = [s]
    while s in states and states[s]["parent"]:
        s = states[s]["parent"]
        out.append(s)
    return out


def rule_for(f, t, on):
    for r in rules:
        if R.matches(r, f, t, on, expand, ancestors):
            return r["id"]
    return None


IDLE = {"closed"}
covered = collections.Counter()
newedges = []
for m in MOVES:
    f, t = canon(m["from"]), canon(m["to"])
    for x in (f, t):
        if x not in states and x not in IDLE and x != "none":
            raise SystemExit(f"a move names {x}, which isn't a state")
    by_edge = any((a, b) in edges for a in ancestors(f) for b in ancestors(t))
    r = rule_for(f, t, m["on"])
    if (f, t) in edges:
        edges[(f, t)].setdefault("seen", set()).add(m["on"])
        covered["edge"] += 1
    elif r:
        covered["rule:" + r] += 1
        report["rule-moves"].append((r, f, t, m["on"]))
    elif by_edge:
        covered["edge (a face of its state)"] += 1
    else:
        newedges.append((m, f, t))

# The moves no edge and no rule covers become edges, drawn from the code.
unplaced = [(m, f, t) for m, f, t in newedges if f not in states or t not in states]
newedges = [(m, f, t) for m, f, t in newedges if f in states and t in states]
for m, f, t in unplaced:
    print("UNPLACED", m["surface"], f, "→", t, "on", m["on"], m["enumerated"])
if "-v" in sys.argv:
    for m, f, t in newedges:
        print("NEW", m["surface"], f, "→", t, "on", m["on"], "|", "; ".join(m["enumerated"])[:150], "|", m["tests"][:1])
for m, f, t in newedges:
    why = [w for w in m["enumerated"] if w not in ("UNLISTED", "fixture")]
    cites = overlay.cites_for(f, t, m, why)
    e = add_edge(f, t, label=overlay.trigger_words(m["on"]), source="core-run", status="provisional", cites=cites)
    e.setdefault("seen", set()).add(m["on"])
    e.setdefault("tests", set()).update(m["tests"][:2])
    covered["new edge"] += 1

for (f, t), (cites, why) in overlay.EDGE_CHANGES.items():
    e = edges[(f, t)]
    e["status"] = "provisional"
    e["cites"] += [c for c in cites if c not in e["cites"]]
    note("edge-changed", (f, t, cites, why))

for (f, t), (cites, why) in overlay.EDGE_GONE.items():
    gone = edges.pop((f, t))
    note("edge-gone", (f, t, cites, why, gone["label"] or gone["trigger"], gone["status"]))

# Bagrat's ruling on the report (B134).
for sid in overlay.RULED_STATES:
    states[sid]["status"] = "ratified"
    states[sid]["cites"].append(overlay.RULING)
    for l in states[sid]["said"]:
        l["copy"] = "ratified"
for e in edges.values():
    if (e["from"], e["to"]) in overlay.RULED_EDGES or "#44" in e["cites"] or (e["from"], e["to"]) == ("paused", "restart-confirm"):
        e["ruled"] = True
assert all(p in edges for p in overlay.RULED_EDGES), "a ruled edge is gone"

for sid, board in overlay.BOARD.items():
    states[sid]["board"] = board

# Kinds and statuses for every edge.
for e in edges.values():
    if e["kind"] == "port":
        e["kind"] = "flow"  # the map's ports were another region's state; the edge now goes there
    if not ({"map", "fixture"} & set(e["source"])):
        e["cites"] += [c for c in overlay.CITES.get((e["from"], e["to"]), []) if c not in e["cites"]]
        e["label"] = e["label"] or overlay.LABELS.get((e["from"], e["to"]), "")
        if not e["label"] and e.get("seen"):
            e["label"] = overlay.trigger_words(sorted(e["seen"])[0])
    if not e["kind"]:
        e["kind"] = overlay.kind_for(e, states)
    if e["status"] is None:
        e["status"] = "provisional" if any(states[x]["status"] == "provisional" for x in (e["from"], e["to"])) else "ratified"
    if "core-run" in e["source"] and not ({"map", "fixture"} & set(e["source"])):
        e["status"] = "provisional"
    e["on"] = sorted(e.pop("seen", []))
    if e.pop("ruled", False):
        e["status"] = "ratified"
        e["cites"].append(overlay.RULING)
    e.pop("tests", None)

# ── regions, notes, the rest of meta ───────────────────────────────────────────────

regions = []
for i in range(1, 7):
    d = FX[i]
    regions.append({
        "id": i,
        "title": overlay.REGION_TITLES[i],
        "heading": d["region"],
        "amendments": d["amendments"],
        "open_notes": d["open_notes"],
        "scaffold_flags": d["scaffold_flags"],
    })

html = (ROOT / "history/spec/UX-MAP.html").read_text()
map_notes = []
for n in MAPJ["notes"]:
    m = re.search(r'<sm-note name="%s"[^>]*>(.*?)</sm-note>' % re.escape(n["name"]), html, re.S)
    text = re.sub(r"\s+", " ", re.sub(r"<[^>]+>", " ", m.group(1))).strip() if m else ""
    about = sorted({x for e in MAPJ["edges"] for a, b in ((e["from"], e["to"]), (e["to"], e["from"])) if a == n["name"] for x in (ends(b) or [])})
    map_notes.append({"id": n["name"], "region": n["region"], "text": text, "about": about})

machine = {
    "about": "The machine: every state, edge, rule, said-line and law of shroom v1 — the spec (DECISIONS B133). Sessions edit it; MACHINE.md and the canvas's machine boards are generated from it.",
    "notation": META["notation"],
    "events": R.EVENTS,
    "laws": META["laws"],
    "ledger": META["ledger"],
    "surfaces": META["surfaces"],
    "absent_from_v1": META["absent_from_v1"],
    "scaffold_flags": META["scaffold_flags"],
    "regions": regions,
    "groups": [{"id": k, **v} for k, v in groups.items()],
    "states": {k: states[k] for k in order},
    "edges": sorted(edges.values(), key=lambda e: (states[e["from"]]["region"], order.index(e["from"]), order.index(e["to"]))),
    "rules": rules,
    "notes": map_notes,
}

# Every state a rule or group names exists.
for g in groups.values():
    for s in g["states"]:
        assert s in states, f"group names {s}"
for r in rules:
    for s in expand(r["from"]) + expand(r.get("to", [])):
        assert s in states or s in IDLE or s in ("none", "*"), f"rule {r['id']} names {s}"

(ROOT / "spec/machine.json").write_text(json.dumps(machine, indent=1, ensure_ascii=False) + "\n")
print(f"{len(states)} states, {len(edges)} edges, {len(rules)} rules, {len(groups)} groups")
print(dict(covered))
report["covered"] = dict(covered)
json.dump({k: v for k, v in report.items()}, open(HERE / "assemble-notes.json", "w"), indent=1, ensure_ascii=False)
