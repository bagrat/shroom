#!/usr/bin/env python3
"""Extract the machine drawn in spec/UX-MAP.html into spec/map.json.

The map is the single drawing of the state machine; its <sm-state>, <sm-port>,
<sm-note> and <sm-edge> tags are the fixture the transition tests check against. Regenerate after
every map edit: python3 tools/extract-map.py
"""
import json
import re
from html.parser import HTMLParser
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent


class MapParser(HTMLParser):
    def __init__(self):
        super().__init__()
        self.region = None
        self.in_h2 = False
        self.h2_text = ""
        self.states, self.ports, self.notes, self.edges = [], [], [], []

    def handle_starttag(self, tag, attrs):
        a = dict(attrs)
        if tag == "h2":
            self.in_h2, self.h2_text = True, ""
        elif tag == "sm-state":
            self.states.append({"name": a["name"], "kind": a.get("kind", "state"), "region": self.region})
        elif tag == "sm-note":
            self.notes.append({"name": a["name"], "region": self.region})
        elif tag == "sm-port":
            self.ports.append({"name": a["name"], "region": self.region})
        elif tag == "sm-edge":
            self.edges.append({
                "from": a["from"], "to": a["to"], "kind": a.get("kind", "flow"),
                "label": a.get("label", ""), "region": self.region,
            })

    def handle_endtag(self, tag):
        if tag == "h2" and self.in_h2:
            self.in_h2 = False
            m = re.match(r"Region (\d)", self.h2_text.strip())
            self.region = int(m.group(1)) if m else None

    def handle_data(self, data):
        if self.in_h2:
            self.h2_text += data


def main():
    p = MapParser()
    p.feed((ROOT / "spec" / "UX-MAP.html").read_text())
    names = {x["name"] for x in p.states + p.ports + p.notes}
    dangling = sorted({e[k] for e in p.edges for k in ("from", "to")} - names)
    out = {"source": "UX-MAP.html", "states": p.states, "ports": p.ports, "notes": p.notes, "edges": p.edges, "dangling_endpoints": dangling}
    (ROOT / "spec" / "map.json").write_text(json.dumps(out, indent=2, ensure_ascii=False) + "\n")
    print(f"{len(p.states)} states, {len(p.ports)} ports, {len(p.notes)} notes, {len(p.edges)} edges, {len(dangling)} dangling endpoints")
    if dangling:
        print("dangling:", ", ".join(dangling))


if __name__ == "__main__":
    main()
