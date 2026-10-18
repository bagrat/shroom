#!/usr/bin/env python3
"""Reduce a recorded run of the core's tests to its distinct moves.

Input: the JSON lines `record-moves.patch` writes (SHROOM_MOVES=<file> cargo test -p
shroom-core --tests), and the MOVE lines the patched enumeration prints (--nocapture).
Output: moves.json — every distinct (surface, from, to, event) the core made, with the arm
that ran, the edge it named, how often, and in which tests. Ids are the code's as recorded.
"""
import collections, json, re, sys

def standing(sc, surface, phase="none"):
    """Where one surface stands: its most specific id, or the tray's state when it's shut."""
    if surface == "popover":
        # With nothing of the tray's open, where the person is: the tray's own state, or
        # the window in front (the tray has no state while a window is open).
        rest = sc["tray"]["state"]
        for win in ("onboarding", "settings", "library"):
            if rest is None and sc[win]:
                rest = standing(sc, win)
        rest = rest or "closed"
        p = sc["popover"]
        if p:
            return p["faces"][0] if p["id"] == "paused" and p["faces"] else p["id"]
        if sc["countdown"]:
            return "countdown"
        # The take's own beats, with the popover shut (enumerate.rs's pop_standing).
        return {"paused": "paused", "stopping": "stopping"}.get(phase, rest)
    if surface == "banner":
        p = sc["popover"]
        return (p.get("banner") or "none") if p else None
    w = sc[surface]
    if not w:
        return None
    sheet = w.get("sheet")
    if sheet:
        return sheet["faces"][-1] if sheet["faces"] else sheet["id"]
    if surface == "onboarding":
        return w["id"]  # its faces are row facts (enumerate.rs's standing)
    return w["faces"][-1] if w["faces"] else w["id"]

def event(ev):
    """The event's kind: its variant, and the action, news or op inside it."""
    m = re.match(r"(\w+)", ev)
    kind = m.group(1)
    if kind == "Act":
        a = re.search(r"action: (\w+)", ev)
        return f"Act({a.group(1)})" if a else kind
    if kind in ("Cloud", "Rec", "Os", "Join"):
        a = re.match(r"\w+\((\w+)", ev)
        return f"{kind}({a.group(1)})" if a else kind
    if kind == "Failed":
        a = re.search(r"op: (\w+)", ev)
        return f"Failed({a.group(1)})"
    if kind == "Key":
        a = re.search(r"key: (\w+)", ev)
        return f"Key({a.group(1)})"
    if kind in ("SecretPasted", "Typed"):
        a = re.search(r"field: (\w+)", ev)
        return f"{kind}({a.group(1)})" if a else kind
    return kind

def main(jsonl, tsv, out):
    moves = {}
    for line in open(jsonl):
        d = json.loads(line)
        test = d["test"].split("::")[-1]
        for s in ("popover", "banner", "onboarding", "library", "settings"):
            pa, pb = d["phase"]
            a, b = standing(d["before"], s, pa), standing(d["after"], s, pb)
            if a is None and b is None:
                continue
            if s == "banner" and (a is None or b is None):
                continue  # the popover opening or closing, not its banner  # the window stays shut: the tray's move, recorded under the popover
            # A window opens from where the person was, and closes to where the tray stands.
            a = a if a is not None else standing(d["before"], "popover", pa)
            b = b if b is not None else standing(d["after"], "popover", pb)
            if a == b:
                continue
            k = (s, a, b, event(d["event"]))
            m = moves.setdefault(k, {"surface": s, "from": a, "to": b, "on": k[3], "arms": collections.Counter(), "edges": set(), "n": 0, "tests": set(), "enumerated": set()})
            m["n"] += 1
            m["arms"][d["arm"]] += 1
            if d["edge"]:
                m["edges"].add(tuple(d["edge"]))
            m["tests"].add(test)
    # The enumeration's own reading of each move it made (the rule or edge that lists it).
    for line in open(tsv):
        _, kind, a, b, why, ev = line.rstrip("\n").split("\t")
        s = {"Onboarding": "onboarding", "Settings": "settings", "Popover": "popover"}[kind]
        k = (s, a, b, event(ev))
        m = moves.setdefault(k, {"surface": s, "from": a, "to": b, "on": k[3], "arms": collections.Counter(), "edges": set(), "n": 0, "tests": set(), "enumerated": set()})
        m["enumerated"].add(why)
    rows = []
    for k in sorted(moves):
        m = moves[k]
        rows.append({"surface": m["surface"], "from": m["from"], "to": m["to"], "on": m["on"], "n": m["n"],
                     "arms": dict(m["arms"]), "edges": sorted(m["edges"]), "enumerated": sorted(m["enumerated"]), "tests": sorted(m["tests"])[:5]})
    json.dump(rows, open(out, "w"), indent=1, ensure_ascii=False)
    print(len(rows), "distinct moves")

if __name__ == "__main__":
    main(*sys.argv[1:4])
