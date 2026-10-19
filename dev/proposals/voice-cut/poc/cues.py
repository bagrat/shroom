#!/usr/bin/env python3
"""Which spoken cues to listen for, measured (spec N65 (1); Bagrat asked to try plain "cut"
and a relaxed set, 2026-10-18, relayed by the orchestrator).

The planner proposes a phrase as a cue only said apart from the speech (shroom-edit's
`find_cues_in`); Clef then says whether the speaker meant it for whoever edits the video
(the tester's /gate, the same question the product would ask). This measures, per phrase:

- misses: takes where the phrase was said as a cue after a slip, in two stock voices, heard
  by Whisper with the mode's prime and with none: did Whisper write it, did the planner
  propose it, and how surely did Clef say it was meant;
- false triggers: every phrase the planner proposes in the 93 transcripts of the planner's
  tests (none of which says any cue but "shroom cut") and in takes where "cut", "redo" and
  the like are said apart but meant for the viewer, with Clef's score for each.

    TESTER_URL_FILE=… python3 cues.py --voices DIR [--out results/cues-….json] [--heard DIR]

Stock voices only (piper); no one's voice is cloned.
"""
import argparse
import json
import os
import subprocess
import sys
import tempfile
import urllib.request
import wave
from pathlib import Path

import numpy as np

ROOT = Path(__file__).resolve().parents[4]
CUT = ROOT / "target/debug/shroom-cut"
TAKES = ROOT / "crates/shroom-edit/tests"
RATE = 16000
VOICES = ["en_US-ryan-medium", "en_US-amy-medium"]

# Said as a cue after a slip: (name, the cue as said).
CUES = [("cut", "Cut."), ("cut-here", "Cut here."), ("cut-that", "Cut that."), ("cut-cut", "Cut, cut."),
        ("scratch-that", "Scratch that."), ("redo", "Redo."), ("let-me-redo-that", "Let me redo that."),
        ("say-that-again", "Let me say that again.")]
SLIP = [("Okay, let's look at the settings.", "keep", 0.7), ("Open the menu and choose the, the share", "flub", 0.7)]
REDO = [("Open the menu and choose Share.", "keep", 0.5), ("That's all there is to it.", "keep", 0)]
# Said apart but meant for the viewer: each line (say, pause after).
MEANT_FOR_VIEWERS = [
    ("tutorial-cut", [("Select the text you want to move.", 0.6), ("Cut.", 0.6), ("Then paste it where you want it.", 0)]),
    ("film-set", [("On a film set, when the scene is over, the director shouts:", 0.4), ("Cut!", 0.6), ("And everyone relaxes.", 0)]),
    ("undo-redo", [("Made a mistake? Click undo.", 0.6), ("Changed your mind?", 0.4), ("Redo.", 0.6), ("It comes right back.", 0)]),
    ("what-it-does", [("What does this button do?", 0.6), ("Cut.", 0.6), ("It takes the clip out and keeps it for later.", 0)]),
    ("shortcut", [("The shortcut is command X.", 0.5), ("Cut.", 0.5), ("And command V to paste.", 0)]),
    ("two-choices", [("You can keep the part, or you can cut it.", 0.6), ("Cut that.", 0.6), ("It's the second choice in the menu.", 0)]),
]
THRESHOLDS = [0.3, 0.4, 0.5, 0.6, 0.7]


def tester():
    base, key = Path(os.environ["TESTER_URL_FILE"]).read_text().strip().split("#k=")
    return base.rstrip("/"), key


def post(path, body, ctype):
    base, key = tester()
    req = urllib.request.Request(base + path, data=body, method="POST",
                                 headers={"x-key": key, "content-type": ctype, "user-agent": "shroom-voice-cut-cues"})
    return json.load(urllib.request.urlopen(req, timeout=180))


def say(voice, text, tmp):
    from piper import PiperVoice  # noqa: imported only to make takes
    v = PiperVoice.load(str(voice))
    f = Path(tmp) / "p.wav"
    with wave.open(str(f), "wb") as w:
        v.synthesize_wav(text, w)
    out = subprocess.run(["ffmpeg", "-hide_banner", "-loglevel", "error", "-i", str(f), "-ar", str(RATE), "-ac", "1", "-f", "f32le", "-"],
                         capture_output=True, check=True).stdout
    return np.frombuffer(out, dtype="<f4").astype(np.float64)


def make(voice, lines):
    """A take from (say, role, pause after) lines, with each line's time, and a room's noise under it."""
    with tempfile.TemporaryDirectory() as tmp:
        parts, script, at = [np.zeros(int(0.4 * RATE))], [], 0.4
        for text, role, pause in lines:
            p = say(voice, text, tmp)
            script.append({"say": text, "role": role, "start": at, "end": at + len(p) / RATE})
            parts += [p, np.zeros(int(pause * RATE))]
            at += len(p) / RATE + pause
    pcm = np.concatenate(parts + [np.zeros(int(0.6 * RATE))])
    rng = np.random.default_rng(len(pcm))
    noise = np.convolve(rng.standard_normal(len(pcm)), np.ones(8) / 8, mode="same")
    noise *= 10 ** (-50 / 20) / (np.sqrt(np.mean(noise ** 2)) + 1e-12)
    return pcm * 0.7 + noise, script


def wav(pcm):
    with tempfile.TemporaryDirectory() as tmp:
        f = Path(tmp) / "t.wav"
        with wave.open(str(f), "wb") as w:
            w.setnchannels(1); w.setsampwidth(2); w.setframerate(RATE)
            w.writeframes((np.clip(pcm, -1, 1) * 32767).astype("<i2").tobytes())
        return f.read_bytes()


def listen(pcm, cache, prime):
    if cache.exists():
        return json.loads(cache.read_text())
    heard = post(f"/listen?cues=relaxed" + ("" if prime else "&prompt=none"), wav(pcm), "audio/wav")
    cache.parent.mkdir(parents=True, exist_ok=True)
    cache.write_text(json.dumps(heard))
    return heard


def plan(t):
    return json.loads(subprocess.run([str(CUT)], input=json.dumps(t), capture_output=True, text=True, check=True).stdout)


ASK = "sharp"


def gate(intents):
    path = f"/gate?ask={ASK}"
    return {m["word"]: m["score"] for m in post(path, json.dumps({"intents": intents}).encode(), "application/json")["meant"]} if intents else {}


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--voices", type=Path, required=True)
    ap.add_argument("--out")
    ap.add_argument("--heard", type=Path, default=Path(tempfile.gettempdir()) / "shroom-cues-heard")
    ap.add_argument("--ask", default="sharp", choices=["first", "sharp"], help="the gate's question: as first asked, or the sharp one")
    a = ap.parse_args()
    global ASK
    ASK = a.ask
    rows = []  # one per phrase proposed or missed: take, phrase, truth, score

    # Misses: each cue after a slip, in each voice, with the mode's prime and with none.
    for name, cue in CUES:
        for voice in VOICES:
            pcm, script = make(a.voices / f"{voice}.onnx", SLIP + [(cue, "cue", 0.9)] + REDO)
            for prime in (True, False):
                heard = listen(pcm, a.heard / f"cue-{name}-{voice}-{'prime' if prime else 'none'}.json", prime)
                t = {"duration": len(pcm) / RATE, "words": heard["words"], "silences": [], "cues": "relaxed"}
                p = plan(t)
                # The cue as said, whatever its times: the take says no other.
                key = lambda x: "".join(ch for ch in x.lower() if ch.isalpha())
                on = [i for i in p["intents"] if key(i["said"]) == key(cue)]
                scores = gate(on)
                rows.append({"kind": "cue", "take": f"{name}/{voice}", "prime": prime, "phrase": name, "heard": heard["text"],
                             "proposed": bool(on), "said": on[0]["said"] if on else None,
                             "score": max(scores.values()) if scores else None})
                print(f"cue  {name:18s} {voice:18s} prime={prime!s:5s} proposed={bool(on)!s:5s} score={rows[-1]['score']}  | {heard['text'][:90]}")

    # False triggers: said apart, meant for the viewer.
    for name, lines in MEANT_FOR_VIEWERS:
        for voice in VOICES:
            pcm, _ = make(a.voices / f"{voice}.onnx", [(s, "keep", p) for s, p in lines])
            heard = listen(pcm, a.heard / f"viewer-{name}-{voice}.json", True)
            p = plan({"duration": len(pcm) / RATE, "words": heard["words"], "silences": [], "cues": "relaxed"})
            scores = gate(p["intents"])
            for i in p["intents"]:
                rows.append({"kind": "viewer", "take": f"{name}/{voice}", "phrase": i["said"], "heard": heard["text"], "score": scores.get(i["word"])})
                print(f"view {name:18s} {voice:18s} {i['said']:14s} score={scores.get(i['word'])}  | {heard['text'][:90]}")
            if not p["intents"]:
                print(f"view {name:18s} {voice:18s} (nothing proposed) | {heard['text'][:90]}")

    # False triggers in every transcript the planner is tested on: no cue there but "shroom cut".
    for f in sorted(list((TAKES / "takes").glob("*/transcript.json")) + list((TAKES / "long").glob("*/transcript.json"))):
        t = json.loads(f.read_text())
        t["cues"] = "relaxed"
        p = plan(t)
        if not p["intents"]:
            continue
        scores = gate(p["intents"])
        for i in p["intents"]:
            rows.append({"kind": "fixture", "take": f.parent.name, "phrase": i["said"], "score": scores.get(i["word"])})
            print(f"fix  {f.parent.name[:40]:40s} {i['said']:14s} score={scores.get(i['word'])}")

    if a.out:
        Path(a.out).write_text(json.dumps({"model": "@cf/cloudflare/clef", "rows": rows}, indent=1))

    cues = [r for r in rows if r["kind"] == "cue"]
    false = [r for r in rows if r["kind"] != "cue"]
    print(f"\n{len(cues)} cue takes, {len(false)} phrases proposed where no cue was said")
    for prime in (True, False):
        cs = [r for r in cues if r["prime"] == prime]
        print(f"prime={prime}: proposed {sum(r['proposed'] for r in cs)}/{len(cs)}")
        for name, _ in CUES:
            pr = [r for r in cs if r["phrase"] == name]
            print(f"   {name:18s} proposed {sum(r['proposed'] for r in pr)}/{len(pr)}  scores {[r['score'] for r in pr]}")
    for th in THRESHOLDS:
        caught = sum(1 for r in cues if r["prime"] and r["score"] is not None and r["score"] >= th)
        wrong = sum(1 for r in false if r["score"] is not None and r["score"] >= th)
        print(f"at {th}: {caught}/{sum(1 for r in cues if r['prime'])} cues kept (primed), {wrong}/{len(false)} false phrases let in")


if __name__ == "__main__":
    sys.exit(main())
