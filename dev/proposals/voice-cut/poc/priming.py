#!/usr/bin/env python3
"""Whether to prime Whisper for "shroom cut", measured (the AI build asked, 2026-10-18).

The space hears each take with Workers AI's Whisper (N66). A prime (`initial_prompt`) can
help it write the cue, but Whisper is known to write its prime into stretches with no
speech, and a false "shroom cut" would cut a viewer's video. For each way of hearing (no
prime, the tester's full prime, the cue alone; each with and without `vad_filter`), this
measures:

- (a) a silent minute (true silence, and a room's noise), (b) a minute of music only
  (synthesized, so no one's recording is used), (c) a talk with no cue (two stock voices):
  any "shroom cut" the planner hears there, and any echo of the prime in what was written;
- (d) the cue takes (the POC's scripts and Bagrat's words, two stock voices, a room's
  noise and a breath after each cue): how many of the cues said the planner hears.

    TESTER_URL_FILE=… python3 -B priming.py --voices DIR [--out results/priming-….json]

Stock voices only (piper); no one's voice is cloned.
"""
import argparse
import json
import os
import subprocess
import sys
import tempfile
import time
import urllib.error
import urllib.request
import wave
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path

import numpy as np

sys.path.insert(0, str(Path(__file__).resolve().parent))
from joins import RATE, SCRIPTS, VOICES, is_cue, make, say  # noqa: E402

ROOT = Path(__file__).resolve().parents[4]
CUT = ROOT / "target/debug/shroom-cut"
# (name, query): how the tester's /listen hears.
WAYS = [("none", "prompt=none"), ("none+vad", "prompt=none&vad=1"), ("full", "cues=shroom"),
        ("full+vad", "cues=shroom&vad=1"), ("cue", "prompt=cue"), ("cue+vad", "prompt=cue&vad=1")]
# What would show the prime written back: the full prime's own words, and the cue.
ECHO = ["shroom", "so, so i", "i mean", "um, uh"]
TALK = [
    ("Welcome back.", 0.6), ("Today we'll look at how to share a screen recording with your team.", 0.7),
    ("First, open the app from the menu bar.", 0.5), ("You'll see the record button and a few settings.", 0.6),
    ("Pick the window you want to share, or the whole screen.", 0.8),
    ("When you're ready, press record, and talk through what you're doing, the way you'd explain it to a colleague sitting next to you.", 0.7),
    ("Take your time.", 0.4), ("Pauses are fine.", 0.9), ("When you're done, press stop.", 0.5),
    ("The link is copied for you, so you can paste it into a message right away.", 0.6),
    ("Anyone with the link can watch it in their browser, with no account and nothing to install.", 0.8),
    ("You can rename the video, add a title, or delete it later from the library.", 0.6),
    ("That's really all there is to it.", 0),
]


def tester():
    base, key = Path(os.environ["TESTER_URL_FILE"]).read_text().strip().split("#k=")
    return base.rstrip("/"), key


def wav(pcm):
    with tempfile.TemporaryDirectory() as tmp:
        f = Path(tmp) / "t.wav"
        with wave.open(str(f), "wb") as w:
            w.setnchannels(1); w.setsampwidth(2); w.setframerate(RATE)
            w.writeframes((np.clip(pcm, -1, 1) * 32767).astype("<i2").tobytes())
        return f.read_bytes()


def listen(body, query, cache):
    if cache.exists():
        return json.loads(cache.read_text())
    base, key = tester()
    req = urllib.request.Request(f"{base}/listen?{query}", data=body, method="POST",
                                 headers={"x-key": key, "content-type": "audio/wav", "user-agent": "shroom-voice-cut-priming"})
    for tries in range(4):
        try:
            heard = json.load(urllib.request.urlopen(req, timeout=180))
            break
        except (OSError, urllib.error.URLError):
            if tries == 3:
                raise
            time.sleep(2 ** tries * 2)
    cache.parent.mkdir(parents=True, exist_ok=True)
    cache.write_text(json.dumps(heard))
    return heard


def heard_cues(words, duration):
    out = subprocess.run([str(CUT)], input=json.dumps({"duration": duration, "words": words, "silences": []}),
                         capture_output=True, text=True, check=True).stdout
    return json.loads(out)["cues"]


def room(rng, n):
    pink = np.convolve(rng.standard_normal(n), np.ones(8) / 8, mode="same")
    pink *= 10 ** (-50 / 20) / (np.sqrt(np.mean(pink ** 2)) + 1e-12)
    return pink + 10 ** (-56 / 20) * np.sqrt(2) * np.sin(2 * np.pi * 50 * np.arange(n) / RATE)


def music(rng, kind, seconds=60.0):
    """A minute of music with no voice: chords, a bass and, for "band", drums and a lead."""
    t = np.arange(int(seconds * RATE)) / RATE
    out = np.zeros_like(t)
    hz = lambda m: 440 * 2 ** ((m - 69) / 12)
    beat = 60 / (96 if kind == "band" else 64)
    bar = 4 * beat
    chords = [[60, 64, 67], [67, 71, 74], [69, 72, 76], [65, 69, 72]]
    for i, start in enumerate(np.arange(0, seconds, bar)):
        seg = (t >= start) & (t < start + bar)
        tt = t[seg] - start
        env = np.minimum(1, tt / (0.02 if kind == "band" else 0.8)) * np.exp(-tt / (2.5 if kind == "band" else 8))
        for m in chords[i % 4]:
            f = hz(m)
            out[seg] += 0.07 * env * (np.sin(2 * np.pi * f * tt) + 0.4 * np.sin(4 * np.pi * f * tt) + 0.2 * np.sin(6 * np.pi * f * tt))
        f = hz(chords[i % 4][0] - 24)
        out[seg] += 0.12 * env * np.sin(2 * np.pi * f * tt)
    if kind == "band":
        n = int(0.25 * RATE)
        k = np.arange(n) / RATE
        for j, at in enumerate(np.arange(0, seconds - 0.3, beat / 2)):
            i0 = int(at * RATE)
            if j % 4 == 0:
                hit = 0.5 * np.sin(2 * np.pi * (50 + 100 * np.exp(-k / 0.03)) * k) * np.exp(-k / 0.12)
            elif j % 4 == 2:
                hit = 0.25 * rng.standard_normal(n) * np.exp(-k / 0.06)
            else:
                hit = 0.06 * np.diff(rng.standard_normal(n + 1)) * np.exp(-k / 0.02)
            out[i0:i0 + n] += hit
        notes = [72, 74, 76, 79, 81, 79, 76, 74]
        for j, at in enumerate(np.arange(0, seconds - beat, beat)):
            seg = (t >= at) & (t < at + beat)
            tt = t[seg] - at
            f = hz(notes[j % len(notes)]) * (1 + 0.004 * np.sin(2 * np.pi * 5.5 * tt))
            out[seg] += 0.09 * np.minimum(1, tt / 0.03) * np.exp(-tt / 0.5) * np.sin(2 * np.pi * f * tt)
    return out * 10 ** (-16 / 20) / (np.sqrt(np.mean(out ** 2)) + 1e-12)


def talk(voice):
    with tempfile.TemporaryDirectory() as tmp:
        parts = [np.zeros(int(0.4 * RATE))]
        for text, pause in TALK:
            parts += [say(voice, text, tmp) * 0.7, np.zeros(int(pause * RATE))]
    pcm = np.concatenate(parts + [np.zeros(int(0.6 * RATE))])
    return pcm + room(np.random.default_rng(len(pcm)), len(pcm))


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--voices", type=Path, required=True)
    ap.add_argument("--out")
    ap.add_argument("--heard", type=Path, default=Path(tempfile.gettempdir()) / "shroom-priming-heard")
    a = ap.parse_args()
    rng = np.random.default_rng(18)
    n = int(60 * RATE)
    # (kind, name, pcm, the cues said in it as (start, end) or None where none is said)
    takes = [("silence", "true", 10 ** (-118 / 20) * rng.standard_normal(n), []),
             ("silence", "room", room(rng, n), []),
             ("music", "band", music(rng, "band"), []),
             ("music", "ambient", music(rng, "ambient"), [])]
    for voice in VOICES:
        takes.append(("talk", voice, talk(a.voices / f"{voice}.onnx"), []))
    for name, script in SCRIPTS.items():
        if name.startswith("6-said-not-meant") or not any(is_cue(s) for s, _ in script):
            continue
        for voice in VOICES:
            pcm, marks = make(a.voices / f"{voice}.onnx", script, rng, "room")
            # The cue ends where its breath begins; the planner's cue must start in the 2 s before.
            takes.append(("cue", f"{name}/{voice}", pcm, [(m["out"][0] - 2.0, m["out"][0] + 0.3) for m in marks]))

    jobs = [(t, way, q) for t in takes for way, q in WAYS]

    def run(job):
        (kind, name, pcm, said), way, q = job
        heard = listen(wav(pcm), q, a.heard / f"{kind}-{name.replace('/', '-')}-{way}.json")
        cues = heard_cues(heard.get("words", []), len(pcm) / RATE)
        text = (heard.get("text") or "").strip()
        hits = sum(any(lo <= c["start"] <= hi for c in cues) for lo, hi in said)
        return {"kind": kind, "take": name, "way": way, "text": text, "cues": len(cues), "said": len(said), "hits": hits,
                "false": len(cues) - hits if said else len(cues), "echo": [e for e in ECHO if e in text.lower()]}

    with ThreadPoolExecutor(3) as pool:
        rows = list(pool.map(run, jobs))
    if a.out:
        Path(a.out).write_text(json.dumps({"model": "@cf/openai/whisper-large-v3-turbo", "ways": dict(WAYS), "rows": rows}, indent=1))
    for r in rows:
        if r["kind"] != "cue":
            print(f"{r['kind']:8s} {r['take']:20s} {r['way']:9s} cues {r['cues']} echo {r['echo'] or '-'} | {r['text'][:110]!r}")
    print()
    for way, _ in WAYS:
        rs = [r for r in rows if r["way"] == way]
        none = [r for r in rs if r["kind"] != "cue"]
        cue = [r for r in rs if r["kind"] == "cue"]
        print(f"{way:9s} no-cue minutes: {sum(r['cues'] for r in none)} \"shroom cut\" heard, {sum(bool(r['echo']) for r in none)} of {len(none)} with an echo, "
              f"{sum(bool(r['text']) for r in none if r['kind'] != 'talk')} of {sum(r['kind'] != 'talk' for r in none)} silent or music minutes with words written; "
              f"cue takes: {sum(r['hits'] for r in cue)} of {sum(r['said'] for r in cue)} cues heard, {sum(r['false'] for r in cue)} heard where none was said")


if __name__ == "__main__":
    sys.exit(main())
