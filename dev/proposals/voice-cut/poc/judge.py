#!/usr/bin/env python3
"""The judgment above the planner, measured (spec N65: mechanism deterministic, judgment above).

For each take: the planner's questions (`shroom-cut`), Clef's score per candidate start (the
tester's /judge, on the test account), and the plans with and without the judgment, both
held to the take's script. A candidate is right when every word of it is a slip the script
says was said again after the cue. Prints a table and a threshold; writes every score.

    TESTER_URL_FILE=… python3 judge.py make --voices DIR      # the new takes, as fixtures
    TESTER_URL_FILE=… python3 judge.py run [--out results.json]

TESTER_URL_FILE names a file holding the tester's link (with its key); the key never goes
on a command line. Stock voices only (piper); no one's voice is cloned.
"""
import argparse
import json
import os
import re
import subprocess
import sys
import tempfile
import urllib.request
import wave
from pathlib import Path

ROOT = Path(__file__).resolve().parents[4]
TAKES = ROOT / "crates/shroom-edit/tests/takes"
CUT = ROOT / "target/debug/shroom-cut"

# The new takes: (name, redone, lines as (say, role, pause after)). `redone`: the slips before
# each cue are said again after it (else the speaker moves on).
SCRIPTS = [
    ("9-clean-reworded", True, [
        ("Hi, this is a quick test of the voice cut.", "keep", 0.9),
        ("Click on this link to see it.", "flub", 0.7),
        ("Shroom cut.", "cue", 1.0),
        ("To get your video, use this link and you'll see it right away.", "keep", 0)]),
    ("10-long-stretch", True, [
        ("Okay, let's set up sharing.", "keep", 0.8),
        ("You open the menu and choose the, the share thing.", "flub", 0.5),
        ("Then you type a name and hit the, uh, the button.", "flub", 0.7),
        ("Shroom cut.", "cue", 1.0),
        ("To share, open the menu and pick Share.", "keep", 0.5),
        ("Type a name and press Enter.", "keep", 0)]),
    ("11-good-then-broken", True, [
        ("Open the settings window.", "keep", 0.8),
        ("Then click the, the blue", "flub", 0.7),
        ("Shroom cut.", "cue", 1.0),
        ("Then click the green button.", "keep", 0)]),
]
VOICES = ["en_US-ryan-medium", "en_US-amy-medium"]
# The existing takes measured beside them, by script (the 7 POC scripts and Bagrat's words).
# A cue with nothing said after it (the end of a take) redoes nothing, whatever the script.
EXISTING = {"1-the-example": True, "2-mid-paragraph": True, "3-mid-sentence": True, "4-no-retake": False,
            "5-two-cues": True, "7-cue-at-the-end": True, "8-reworded-redo": True}


def tester():
    base, key = Path(os.environ["TESTER_URL_FILE"]).read_text().strip().split("#k=")
    return base.rstrip("/"), key


def post(path, body, ctype):
    base, key = tester()
    req = urllib.request.Request(base + path, data=body, method="POST",
                                 headers={"x-key": key, "content-type": ctype, "user-agent": "shroom-voice-cut-judge"})
    return json.load(urllib.request.urlopen(req, timeout=180))


def make(voices_dir):
    from piper import PiperVoice  # noqa: imported only to make takes
    for name, _, lines in SCRIPTS:
        for voice in VOICES:
            v = PiperVoice.load(str(Path(voices_dir) / f"{voice}.onnx"))
            with tempfile.TemporaryDirectory() as tmp:
                parts, script, at = [], [], 0.0
                for k, (say, role, pause) in enumerate(lines):
                    f = Path(tmp) / f"p{k}.wav"
                    with wave.open(str(f), "wb") as w:
                        v.synthesize_wav(say, w)
                    with wave.open(str(f)) as w:
                        length = w.getnframes() / w.getframerate()
                    parts.append((f, pause))
                    script.append({"say": say, "role": role, "pause": pause, "start": round(at, 3), "end": round(at + length, 3)})
                    at += length + pause
                take = Path(tmp) / "take.wav"
                args, graph = [], ""
                for k, (f, pause) in enumerate(parts):
                    args += ["-i", str(f)]
                inputs = len(parts)
                for k, (f, pause) in enumerate(parts):
                    graph += f"[{k}:a]aresample=16000,aformat=channel_layouts=mono[a{k}];"
                    if pause:
                        args += ["-f", "lavfi", "-t", str(pause), "-i", "anullsrc=r=16000:cl=mono"]
                        graph += f"[{inputs}:a]aformat=channel_layouts=mono[s{k}];"
                        inputs += 1
                order = "".join(f"[a{k}]" + (f"[s{k}]" if p else "") for k, (_, p) in enumerate(parts))
                n = order.count("[")
                subprocess.run(["ffmpeg", "-hide_banner", "-loglevel", "error", "-y", *args, "-filter_complex",
                                f"{graph}{order}concat=n={n}:v=0:a=1[o]", "-map", "[o]", "-ar", "16000", "-ac", "1", str(take)], check=True)
                heard = post("/listen", take.read_bytes(), "audio/wav")
                err = subprocess.run(["ffmpeg", "-hide_banner", "-i", str(take), "-af", "silencedetect=noise=-38dB:d=0.12", "-f", "null", "-"],
                                     capture_output=True, text=True).stderr
                starts = [float(x) for x in re.findall(r"silence_start: (-?[\d.]+)", err)]
                ends = [float(x) for x in re.findall(r"silence_end: ([\d.]+)", err)]
                if len(starts) > len(ends):
                    ends.append(at)
                out = TAKES / f"wai-{voice}-{name}"
                out.mkdir(exist_ok=True)
                (out / "transcript.json").write_text(json.dumps({
                    "duration": round(at, 3),
                    "words": [{"text": w["text"], "start": w["start"], "end": w["end"]} for w in heard["words"]],
                    "silences": [{"start": max(0.0, s), "end": e} for s, e in zip(starts, ends)]}, indent=1))
                (out / "script.json").write_text(json.dumps(script, indent=1))
                print(out.name, "—", heard["text"])


def plan(transcript, answers=None):
    body = dict(transcript, **({"answers": answers} if answers else {}))
    return json.loads(subprocess.run([str(CUT)], input=json.dumps(body), capture_output=True, text=True, check=True).stdout)


def role_at(lines, t):
    return next((l["role"] for l in lines if l["start"] - 0.25 <= t <= l["end"] + 0.25), None)


def judge_takes():
    names = []
    for d in sorted(TAKES.iterdir()):
        if not d.name.startswith("wai-"):
            continue
        script = d.name.split("-medium-", 1)[1]
        redone = EXISTING.get(script, next((r for n, r, _ in SCRIPTS if n == script), None))
        if redone is not None:
            names.append((d, redone))
    return names


def held(lines, cuts, words):
    """The script's verdict on a plan: a kept line cut into, a slip left."""
    applied = [(c["start"], c["end"]) for c in cuts if c["applied"]]
    offered = [(c["start"], c["end"]) for c in cuts if not c["applied"]]
    cut = lambda l, rs: sum(max(0.0, min(l["end"], e) - max(l["start"], s)) for s, e in rs)
    bad = []
    for l in lines:
        length = l["end"] - l["start"]
        if l["role"] == "keep" and cut(l, applied) > 0.08:
            bad.append("cut into a kept line")
        if l["role"] == "flub" and cut(l, applied) + cut(l, offered) < 0.9 * length:
            bad.append("left a slip")
        if l["role"] == "flub" and cut(l, applied) < 0.9 * length:
            bad.append("a slip only offered")
    return bad


def run(out_path):
    rows, log = [], []
    for d, redone in judge_takes():
        t = json.loads((d / "transcript.json").read_text())
        lines = json.loads((d / "script.json").read_text())
        own = plan(t)
        qs = own["questions"]
        answers = post("/judge", json.dumps({"questions": qs}).encode(), "application/json")["answers"] if qs else []
        judged = plan(t, answers)
        words = t["words"]
        for q in qs:
            a = next((a for a in answers if a["cue"] == q["cue"]), {"scores": []})
            for k, (cand, score) in enumerate(zip(q["candidates"], a["scores"])):
                n = len(cand.split())
                # The candidate's words are the n words before the cue's first word.
                cue_words = [i for i, w in enumerate(words) if re.sub(r"\W", "", w["text"].lower()) in ("shroom", "troom", "drum")]
                right = None
                for ci in cue_words:
                    span = words[max(0, ci - n):ci]
                    if " ".join(w["text"] for w in span) == cand:
                        roles = {role_at(lines, (w["start"] + w["end"]) / 2) for w in span}
                        right = redone and bool(q["after"].strip()) and roles <= {"flub"}
                        break
                rows.append((d.name, q["cue"], k, score, right, cand))
        log.append({"take": d.name, "questions": qs, "answers": answers,
                    "own": [c["kind"] + (" applied" if c["applied"] else " offered") + ": " + c["removed"] for c in own["edit_list"]["cuts"]],
                    "judged": [c["kind"] + (" applied" if c["applied"] else " offered") + ": " + c["removed"] for c in judged["edit_list"]["cuts"]],
                    "own_held": held(lines, own["edit_list"]["cuts"], words), "judged_held": held(lines, judged["edit_list"]["cuts"], words)})
    pos = sorted(s for *_, s, r, _ in [(r[0], r[1], r[2], r[3], r[4], r[5]) for r in rows] if r is True)
    neg = sorted(s for *_, s, r, _ in [(r[0], r[1], r[2], r[3], r[4], r[5]) for r in rows] if r is False)
    print(f"{len(rows)} candidates: {len(pos)} right, {len(neg)} wrong")
    print(f"right: min {pos[0]:.3f}  median {pos[len(pos) // 2]:.3f}" if pos else "right: none")
    print(f"wrong: max {neg[-1]:.3f}  median {neg[len(neg) // 2]:.3f}" if neg else "wrong: none")
    for th in (0.3, 0.4, 0.5, 0.6, 0.7):
        print(f"  at {th}: {sum(p >= th for p in pos)}/{len(pos)} right taken, {sum(n >= th for n in neg)}/{len(neg)} wrong taken")
    worse = [l for l in log if any(b == "cut into a kept line" for b in l["judged_held"])]
    better = [l for l in log if "a slip only offered" in l["own_held"] and "a slip only offered" not in l["judged_held"]]
    print(f"the judgment cut into a kept line in {len(worse)} takes; it turned an offered slip into a cut in {len(better)}")
    for l in worse:
        print("  cut into a kept line:", l["take"], l["judged"])
    for name, cue, k, score, right, cand in rows:
        mark = {True: "RIGHT", False: "wrong", None: "?"}[right]
        print(f"  {score:5.3f} {mark:5s} {name[4:40]:36s} cue {cue} #{k + 1} {cand[:80]}")
    if out_path:
        Path(out_path).write_text(json.dumps({"model": "@cf/cloudflare/clef", "rows": [
            {"take": n, "cue": c, "candidate": k, "score": s, "right": r, "text": t} for n, c, k, s, r, t in rows], "takes": log}, indent=1))


if __name__ == "__main__":
    ap = argparse.ArgumentParser()
    ap.add_argument("what", choices=["make", "run"])
    ap.add_argument("--voices", type=Path)
    ap.add_argument("--out")
    a = ap.parse_args()
    if a.what == "make":
        make(a.voices)
    else:
        run(a.out)
