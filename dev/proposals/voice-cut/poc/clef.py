#!/usr/bin/env python3
"""Does Clef-flash (Workers AI's decision model) tell a spoken cut cue from talk about one?

Every place in the fixtures where the code word (or a sound-alike) meets "cut", labelled by
the script (a cue line or a mention line), plus hand-written sentences that use "cut" and
"shroom" in ordinary speech. Each becomes one `noul` question over its context; the answer
is Clef's chance that it's an instruction to cut. Prints the separation and a threshold.

    WAI_URL=… WAI_KEY=… python3 clef.py crates/shroom-edit/tests/takes [model]
"""
import json
import os
import re
import sys
import urllib.request
from pathlib import Path

CUE = re.compile(r"^(s?ch?r|t?r|d?r)\w*?(oo|u|ew)\w*$|^shrooms?$", re.I)

HAND = [
    # (text, is an instruction to cut)
    ("Okay so the settings live here. Shroom cut. Okay so the settings live in this tab.", True),
    ("And then you press, um, no wait. Shroom cut. And then you press the big blue button.", True),
    ("That came out wrong, shroom cut, let me say that again.", True),
    ("Uh, shroom cut.", True),
    ("Welcome back everyone, today we are, hmm. Shroom cut. Welcome back, everyone.", True),
    ("If you fluff a line, just say shroom cut and say it again.", False),
    ("The shroom cut feature removes your mistakes.", False),
    ("We added shroom cut in the last release, and people love it.", False),
    ("You can cut the start of a video in the editor.", False),
    ("I cut my finger on the shroom box yesterday.", False),
    ("Shroom, cut the long silence at the start, it's boring.", False),
    ("Every time I say the words shroom cut, the app trims the slip.", False),
    ("Let me show you how cut and paste works here.", False),
    ("Drum cut loops are what I use for the intro music.", False),
    ("The budget cut hit the shroom team hard.", False),
]


def candidates(root):
    out = []
    for d in sorted(Path(root).iterdir()):
        t = json.loads((d / "transcript.json").read_text())
        lines = json.loads((d / "script.json").read_text())
        ws = t["words"]
        for i in range(len(ws) - 1):
            a = re.sub(r"\W", "", ws[i]["text"].lower())
            b = re.sub(r"\W", "", ws[i + 1]["text"].lower())
            joined = a.endswith("cut") and CUE.match(a[:-3] or "x")
            if not (joined or (CUE.match(a or "x") and b in ("cut", "cuts", "kut", "cot"))):
                continue
            mid = (ws[i]["start"] + ws[i + 1]["end"]) / 2
            role = next((l["role"] for l in lines if l["start"] - 0.4 <= mid <= l["end"] + 0.4), None)
            if role not in ("cue", "mention"):
                continue
            ctx = " ".join(w["text"] for w in ws[max(0, i - 14): i + 14])
            out.append((d.name, ctx, " ".join(w["text"] for w in ws[i:i + 2]), role == "cue"))
    return out


def ask(batch, model):
    state = ("Transcript snippets from screen recordings made with shroom. In shroom, a speaker who slips "
             "says the command \"shroom cut\" on its own, then says the line again; the editor then cuts "
             "the slip and the command. The transcriber may mishear it as \"Troom cut\" or \"drum cut\". "
             "People also talk about the feature (\"just say shroom cut\", \"the shroom cut feature\") "
             "or use the word cut in other ways; those are not commands. Each numbered snippet is "
             "independent.\n\n") + \
        "\n".join(f"Snippet {k + 1}: {c[1]}" for k, c in enumerate(batch))
    questions = {f"q{k + 1}": {"type": "noul", "instructions":
                 f"In snippet {k + 1}, is the phrase \"{c[2]}\" the speaker telling the video editor to cut out "
                 f"a mistake they just made (a spoken editing command), rather than talking about the phrase, "
                 f"a feature, or cutting something else?"} for k, c in enumerate(batch)}
    body = {"run_model": model, "model": model.split("/")[-1], "state": state, "questions": questions}
    req = urllib.request.Request(os.environ["WAI_URL"], data=json.dumps(body).encode(),
                                 headers={"content-type": "application/json", "user-agent": "shroom-poc/0",
                                          "x-poc-key": os.environ["WAI_KEY"]})
    r = json.loads(urllib.request.urlopen(req, timeout=120).read())
    if "error" in r:
        raise SystemExit(r["error"])
    out = r["out"]
    return [out["answers"][f"q{k + 1}"]["noul"] for k in range(len(batch))], r["ms"]


def main():
    root, model = sys.argv[1], (sys.argv[2] if len(sys.argv) > 2 else "@cf/cloudflare/clef-flash")
    cands = candidates(root) + [("hand", t, "shroom cut" if "shroom cut" in t.lower() else "cut", y) for t, y in HAND]
    scores, ms = [], 0
    batch = int(os.environ.get("BATCH", "32"))
    for i in range(0, len(cands), batch):
        s, m = ask(cands[i:i + batch], model)
        scores += s
        ms += m
    pos = sorted(p for p, c in zip(scores, cands) if c[3])
    neg = sorted(p for p, c in zip(scores, cands) if not c[3])
    print(f"{model}: {len(pos)} cues, {len(neg)} not cues, {ms} ms in all")
    print(f"cues: min {pos[0]:.3f}  median {pos[len(pos) // 2]:.3f}")
    print(f"not:  max {neg[-1]:.3f}  median {neg[len(neg) // 2]:.3f}")
    for th in (0.3, 0.5, 0.7):
        tp = sum(p >= th for p in pos)
        fp = sum(p >= th for p in neg)
        print(f"  at {th}: {tp}/{len(pos)} cues kept, {fp}/{len(neg)} non-cues let through")
    print("hardest:")
    for p, c in sorted(zip(scores, cands), key=lambda x: abs(x[0] - (1 if x[1][3] else 0)), reverse=True)[:8]:
        print(f"  {p:.3f} {'CUE' if c[3] else 'not'} {c[0][:28]:28s} {c[1][:110]}")


if __name__ == "__main__":
    main()
