#!/usr/bin/env python3
"""Render spec/machine.json (the machine, DECISIONS B133) as spec/MACHINE.md: the readable
form, for reading and for PR review. Nobody edits MACHINE.md; CI fails when it is stale.

    python3 tools/render-machine-md.py           # writes spec/MACHINE.md
    python3 tools/render-machine-md.py --check   # exit 1 if spec/MACHINE.md is stale
"""
import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
SRC = ROOT / "spec" / "machine.json"
OUT = ROOT / "spec" / "MACHINE.md"


def cell(text):
    return str(text).replace("|", "\\|").replace("\n", "<br>")


def cites(cs):
    return ", ".join(cs) if cs else ""


def render(m):
    states = m["states"]
    out = []
    w = out.append
    w("<!-- Generated from spec/machine.json by tools/render-machine-md.py. Don't edit: change the JSON and re-run. -->")
    w("")
    w("# The machine")
    w("")
    w(m["about"])
    w("")
    w(f"{len(states)} states · {len(m['edges'])} edges · {len(m['rules'])} rules · "
      f"{sum(len(s['said']) for s in states.values())} said-lines · {len(m['laws'])} laws · {len(m['ledger'])} ledger rows.")
    w("")
    w("Status: **ratified** is Bagrat's; **provisional** is designed or built and awaits him (each cites its row: "
      "`#n` is a DEVIATIONS row of docs/DECISIONS.md, `Bn` a decision, `Nn`/`Un`/`Pn` a ledger row below). "
      "`SM:n` in older citations is a line of the retired spec text, kept in history/spec/UX-STATEMACHINE.md.")
    w("")
    w("## Contents")
    w("")
    for r in m["regions"]:
        w(f"- [Region {r['id']} — {r['title']}](#region-{r['id']})")
    w("- [Rules](#rules) · [Groups](#groups) · [Laws](#laws) · [The ledger](#the-ledger) · [Surfaces](#surfaces) · "
      "[Absent from v1](#absent-from-v1) · [Scaffold flags](#scaffold-flags) · [Notes](#notes) · [Events](#events)")
    w("")
    w("## Notation")
    w("")
    w(m["notation"])
    w("")
    out_edges = {}
    for e in m["edges"]:
        out_edges.setdefault(e["from"], []).append(e)
    for r in m["regions"]:
        w(f'<a id="region-{r["id"]}"></a>')
        w("")
        w(f"## Region {r['id']} — {r['title']}")
        w("")
        w(f"*{r['heading']}*")
        w("")
        for sid, s in states.items():
            if s["region"] != r["id"]:
                continue
            w(f"### `{sid}`")
            w("")
            facts = [s["kind"], s["surface"], f"**{s['status']}**"]
            if s.get("parent"):
                facts.append(f"a face of `{s['parent']}`")
            if s.get("board"):
                facts.append(f"board: {s['board']}")
            if s.get("cites"):
                facts.append("cites " + cites(s["cites"]))
            w(" · ".join(facts))
            w("")
            w(s["summary"])
            w("")
            if s["entry"]:
                w(f"**Entry.** {s['entry']}")
                w("")
            if s["said"]:
                w("| Said | Role | Copy | Note |")
                w("|---|---|---|---|")
                for l in s["said"]:
                    note = l.get("note", "")
                    if l.get("was"):
                        note = (note + " · " if note else "") + f"was “{l['was']}”"
                    if l.get("cites"):
                        note = (note + " · " if note else "") + cites(l["cites"])
                    w(f"| “{cell(l['text'])}” | {l['role']} | {l['copy']} | {cell(note)} |")
                w("")
            if s["actions"]:
                w("| Action | Kind | Return | Goes to | Note |")
                w("|---|---|---|---|---|")
                for a in s["actions"]:
                    w(f"| {cell(a['label'])} | {a['kind']} | {'↵' if a.get('return_default') else ''} | "
                      f"{('`' + a['goes_to'] + '`') if a.get('goes_to') else ''} | {cell(a.get('note', ''))} |")
                w("")
            if out_edges.get(sid):
                w("| → | Kind | Label | Status | Cites |")
                w("|---|---|---|---|---|")
                for e in out_edges[sid]:
                    label = e["label"] or e["trigger"]
                    w(f"| `{e['to']}` | {e['kind']} | {cell(label)} | {e['status']} | {cites(e['cites'])} |")
                w("")
        if r["amendments"]:
            w(f"#### Region {r['id']}'s amendments")
            w("")
            for a in r["amendments"]:
                w(f"- **{a['ledger_id']}** (amends {a['amends']}): {a['summary']}")
            w("")
        if r["open_notes"]:
            w(f"#### Region {r['id']}'s open notes")
            w("")
            for n in r["open_notes"]:
                w(f"- {n}")
            w("")
        if r["scaffold_flags"]:
            w(f"#### Region {r['id']}'s scaffold flags")
            w("")
            for f in r["scaffold_flags"]:
                w(f"- {f['flag']}")
            w("")
    w("## Rules")
    w("")
    w("A move that holds for a whole group of states, written once (B133). The tests expand each into its moves.")
    w("")
    for r in m["rules"]:
        ends = f"{', '.join(r['from'])} → {', '.join(r['to'])}"
        extra = " (and back)" if r.get("reverse") else (" (a state and its own faces)" if r.get("family") else "")
        w(f"- **{r['label']}** (`{r['id']}`, {r['kind']}, **{r['status']}**, {cites(r['cites'])}): {ends}{extra}, on {', '.join(r['on'])}. {r['says']}")
    w("")
    w("## Groups")
    w("")
    for g in m["groups"]:
        w(f"- **@{g['id']}** — {g['title']} ({cites(g['cites'])}): " + ", ".join(f"`{s}`" for s in g["states"]))
    w("")
    w("## Laws")
    w("")
    for l in m["laws"]:
        w(f"{l['n']}. {l['rule']}")
        for i in l["invariants"]:
            w(f"   - {i}")
    w("")
    w("## The ledger")
    w("")
    w("| Id | Question | Decision | Status | Date |")
    w("|---|---|---|---|---|")
    for row in m["ledger"]:
        w(f"| {row['id']} | {cell(row['title'])} | {cell(row['decision'])} | {row['status']} | {row['date']} |")
    w("")
    w("## Surfaces")
    w("")
    for s in m["surfaces"]:
        w(f"- **{s['name']}** — {s['may_do']}")
    w("")
    w("## Absent from v1")
    w("")
    for a in m["absent_from_v1"]:
        w(f"- {a}")
    w("")
    w("## Scaffold flags")
    w("")
    for f in m["scaffold_flags"]:
        w(f"- {f}")
    w("")
    w("## Notes")
    w("")
    w("The map's notes: lines about a state that aren't a state.")
    w("")
    for n in m["notes"]:
        about = ", ".join(f"`{s}`" for s in n["about"])
        w(f"- **{n['id']}** (Region {n['region']}{', about ' + about if about else ''}): {n['text']}")
    w("")
    w("## Events")
    w("")
    w(m["events"]["about"])
    w("")
    for k, v in m["events"].items():
        if k != "about":
            w(f"- **{k}**: {', '.join(v)}")
    return "\n".join(out) + "\n"


def main():
    text = render(json.loads(SRC.read_text()))
    if "--check" in sys.argv:
        if not OUT.exists() or OUT.read_text() != text:
            print("spec/MACHINE.md is stale: run python3 tools/render-machine-md.py", file=sys.stderr)
            sys.exit(1)
        print("spec/MACHINE.md is current")
        return
    OUT.write_text(text)
    print(f"spec/MACHINE.md: {len(text.splitlines())} lines")


if __name__ == "__main__":
    main()
