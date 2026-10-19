#!/usr/bin/env python3
"""The joins a voice cut leaves, measured (spec N65: the edges set in the quiet).

Bagrat heard the tails of cut words and glitches at the joins on the tester (2026-10-18).
For takes of his scripts in stock voices, with a room's noise under them and some words
run straight on into the next (so no silence lies between), this runs the planner with and
without the take's loudness, renders what plays with crossfades of several lengths, and
measures at each join:

- whether the edge sits in speech (the loudness there over the take's quiet), and
- the click: the join's sharpest change against the sound around it (a ratio; about 1 is
  as smooth as the room itself), and how much speech a crossfade dims.

    TESTER_URL_FILE=… python3 joins.py --voices DIR [--out results/joins-….json]

With --breath (Bagrat heard a breath play after a cue's cut, 2026-10-18), each cue is
followed by what a speaker leaves there: the word's release and a breath out, a pause, and
a breath in before the next line; once as a browser hears it (its noise suppression leaves
true silence between words) and once with a room's noise. It measures, for each planner
given with --cut: how much of the breath out still plays, and how much of the breath in
before the next line is cut.

The words come from Whisper on the test account (the tester's /listen); stock voices only.
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
TAKES = ROOT / "crates/shroom-edit/tests/takes"
RATE = 16000

# (say, pause after): a pause of 0.05 s runs the word straight on into the next.
SCRIPTS = {
    "bagrat-1": [("Hi, this is a quick test of shroom.", 0.8), ("Today, I'll show you how to share, uh, how to", 0.6),
                 ("shroom cut.", 1.0), ("Today, I'll show you how to share a recording in one click.", 0.7),
                 ("You press stop and the link is pasted.", 0.5), ("Shroom cut.", 0.9),
                 ("When you want to get your link, just press stop and your link is", 0.6), ("Shroom cut.", 0.9),
                 ("Okay. That's it.", 0)],
    "bagrat-2": [("Hi, this is a quick test of shroom.", 0.8), ("Today I will show you how to share a recording in one link.", 0.4),
                 ("No. Shroom cut.", 0.05), ("Today I'll show you how to share a recording in one click.", 0.05),
                 ("You press stop and the link is pasted,", 0.4), ("shroom cut,", 0.05),
                 ("when you want to get your link just press stop and the link will be copied to your clipboard and that's it.", 0)],
}
# And the POC's scripts, as their fixtures list their lines and pauses.
for f in sorted(TAKES.glob("wai-en_US-ryan-medium-*/script.json")):
    SCRIPTS[f.parent.name.split("-medium-", 1)[1]] = [(l["say"], l.get("pause", 0.5)) for l in json.loads(f.read_text())]
VOICES = ["en_US-ryan-medium", "en_US-amy-medium"]
FADES = [0.0, 0.01, 0.02, 0.04, 0.06, 0.08, 0.12]


def tester():
    base, key = Path(os.environ["TESTER_URL_FILE"]).read_text().strip().split("#k=")
    return base.rstrip("/"), key


def listen(pcm, cache):
    """Whisper's words for a take, asked once: a rerun with other fades hears the same."""
    if cache.exists():
        return json.loads(cache.read_text())
    base, key = tester()
    buf = (np.clip(pcm, -1, 1) * 32767).astype("<i2").tobytes()
    with tempfile.TemporaryDirectory() as tmp:
        f = Path(tmp) / "t.wav"
        with wave.open(str(f), "wb") as w:
            w.setnchannels(1); w.setsampwidth(2); w.setframerate(RATE); w.writeframes(buf)
        body = f.read_bytes()
    req = urllib.request.Request(base + "/listen", data=body, method="POST",
                                 headers={"x-key": key, "content-type": "audio/wav", "user-agent": "shroom-voice-cut-joins"})
    heard = json.load(urllib.request.urlopen(req, timeout=180))
    cache.parent.mkdir(parents=True, exist_ok=True)
    cache.write_text(json.dumps(heard))
    return heard


def say(voice, text, tmp):
    from piper import PiperVoice  # noqa: imported only to make takes
    v = PiperVoice.load(str(voice))
    f = Path(tmp) / "p.wav"
    with wave.open(str(f), "wb") as w:
        v.synthesize_wav(text, w)
    out = subprocess.run(["ffmpeg", "-hide_banner", "-loglevel", "error", "-i", str(f), "-ar", str(RATE), "-ac", "1", "-f", "f32le", "-"],
                         capture_output=True, check=True).stdout
    return np.frombuffer(out, dtype="<f4").astype(np.float64)


def is_cue(text):
    t = text.lower()
    return "shroom cut" in t or "strum cut" in t


def breath(rng, seconds, peak_db, rise, fall):
    """Noise shaped like a breath: up over `rise` seconds, then dying away over `fall`."""
    n = int(seconds * RATE)
    k = np.arange(n) / RATE
    x = np.convolve(rng.standard_normal(n), np.ones(4) / 4, mode="same")
    x *= np.minimum(1, k / rise) * np.exp(-np.maximum(0, k - rise) / fall)
    return x * 10 ** (peak_db / 20) / (np.max(np.abs(x)) + 1e-12)


def make(voice, script, rng, breaths=None):
    """A take of `script`. With `breaths` ("browser" or "room"), each cue is followed by a
    breath out, the pause, and a breath in before the next line; their times are returned."""
    marks = []
    with tempfile.TemporaryDirectory() as tmp:
        parts, at = [np.zeros(int(0.4 * RATE))], 0.4
        for text, pause in script:
            p = say(voice, text, tmp) * 0.7
            parts.append(p)
            at += len(p) / RATE
            if breaths and is_cue(text):
                out = breath(rng, 0.55, -30, 0.03, 0.09)
                gap = np.zeros(int(max(pause, 0.35) * RATE))
                inn = breath(rng, 0.35, -70, 0.3, 1.0)
                parts += [out, gap, inn]
                marks.append({"out": (at, at + len(out) / RATE), "in": (at + (len(out) + len(gap)) / RATE, at + (len(out) + len(gap) + len(inn)) / RATE)})
                at += (len(out) + len(gap) + len(inn)) / RATE
            else:
                parts.append(np.zeros(int(pause * RATE)))
                at += pause
    pcm = np.concatenate(parts + [np.zeros(int(0.6 * RATE))])
    if breaths == "browser":
        # Noise suppression: true silence between words, as Bagrat's browser gave (-118 dB).
        return pcm + 10 ** (-118 / 20) * rng.standard_normal(len(pcm)), marks
    # A room under it: pinkish noise about -50 dBFS and a little hum, as a laptop's microphone hears.
    white = rng.standard_normal(len(pcm))
    pink = np.convolve(white, np.ones(8) / 8, mode="same")
    pink *= 10 ** (-50 / 20) / (np.sqrt(np.mean(pink ** 2)) + 1e-12)
    hum = 10 ** (-56 / 20) * np.sqrt(2) * np.sin(2 * np.pi * 50 * np.arange(len(pcm)) / RATE)
    return pcm + pink + hum, marks


def kept(keep, span):
    """How much of `span` (seconds) still plays."""
    return sum(max(0.0, min(e, span[1]) - max(s, span[0])) for s, e in keep)


def loudness(pcm):
    n = len(pcm) // 160
    frames = pcm[: n * 160].reshape(n, 160)
    return 10 * np.log10(np.mean(frames ** 2, axis=1) + 1e-12)


def silences(db):
    floor = np.sort(db)[int(len(db) * 0.1)] if len(db) else -60
    thr = min(-30, floor + 10)
    out, start = [], None
    for k, d in enumerate(db):
        if d < thr:
            start = k if start is None else start
        elif start is not None:
            if (k - start) * 0.01 >= 0.12:
                out.append({"start": start * 0.01, "end": k * 0.01})
            start = None
    if start is not None and (len(db) - start) * 0.01 >= 0.12:
        out.append({"start": start * 0.01, "end": len(db) * 0.01})
    return out, thr


def plan(t, cut=CUT):
    return json.loads(subprocess.run([str(cut)], input=json.dumps(t), capture_output=True, text=True, check=True).stdout)


def render(pcm, keep, fade):
    """What plays: the kept stretches end to end, each join an equal-power crossfade."""
    F = int(round(fade * RATE))
    parts = [(int(round(s * RATE)), min(len(pcm), int(round(e * RATE)))) for s, e in keep]
    parts = [(a, b) for a, b in parts if b > a]
    out, joins = np.zeros(0), []
    for i, (a, b) in enumerate(parts):
        seg = pcm[a:b].copy()
        f = min(F, len(seg) // 2, len(out) // 2) if i else 0
        if i:
            joins.append(len(out) - f // 2)
        if f:
            th = (np.arange(f) + 0.5) / f * (np.pi / 2)
            out[-f:] = out[-f:] * np.cos(th) + seg[:f] * np.sin(th)
            seg = seg[f:]
        out = np.concatenate([out, seg])
    return out, joins


def click(out, j):
    """The join's sharpest change (the first difference, 4 ms around it) against the typical
    change over the 300 ms around it."""
    d = np.abs(np.diff(out))
    w = int(0.002 * RATE)
    near = d[max(0, j - w): j + w]
    around = np.concatenate([d[max(0, j - int(0.15 * RATE)): max(0, j - w)], d[j + w: j + int(0.15 * RATE)]])
    if len(near) == 0 or len(around) == 0:
        return None
    return float(np.max(near) / (np.percentile(around, 90) + 1e-9))


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--voices", type=Path, required=True)
    ap.add_argument("--out")
    ap.add_argument("--heard", type=Path, default=Path(tempfile.gettempdir()) / "shroom-joins-heard", help="where Whisper's answers are kept between runs")
    ap.add_argument("--breath", action="store_true", help="a breath after each cue, as a browser and a room hear it")
    ap.add_argument("--cut", action="append", type=Path, help="a planner to measure (more than one to compare); the built one by default")
    a = ap.parse_args()
    if a.breath:
        return breaths(a)
    rng = np.random.default_rng(18)
    rows = []
    for name, script in SCRIPTS.items():
        for voice in VOICES:
            pcm, _ = make(a.voices / f"{voice}.onnx", script, rng)
            heard = listen(pcm, a.heard / f"{name}-{voice}.json")
            db = loudness(pcm)
            sil, thr = silences(db)
            base = {"duration": len(pcm) / RATE, "words": heard["words"], "silences": sil}
            for levels in (False, True):
                t = dict(base, **({"levels": {"step": 0.01, "db": [round(float(x), 1) for x in db]}} if levels else {}))
                p = plan(t)
                cuts = [c for c in p["edit_list"]["cuts"] if c["applied"]]
                edges = [x for c in cuts for x in (c["start"], c["end"])]
                in_speech = [float(db[min(len(db) - 1, int(x / 0.01))]) > thr + 6 for x in edges]
                row = {"take": f"{name}/{voice}", "levels": levels, "heard": heard["text"], "cuts": [c["removed"] for c in cuts],
                       "edges": len(edges), "edges_in_speech": int(sum(in_speech)), "clicks": {}, "speech_dimmed": {}}
                for fade in FADES:
                    out, joins = render(pcm, p["keep"], fade)
                    cs = [c for c in (click(out, j) for j in joins) if c is not None]
                    row["clicks"][str(fade)] = {"median": float(np.median(cs)) if cs else None, "max": float(np.max(cs)) if cs else None}
                    # Speech a crossfade dims: of the samples it fades, the share that was speech.
                    F = int(round(fade * RATE))
                    faded = [db[min(len(db) - 1, int((j / RATE) / 0.01))] > thr + 6 for j in joins] if F else []
                    row["speech_dimmed"][str(fade)] = float(sum(faded) / len(faded)) if faded else 0.0
                rows.append(row)
                print(f"{row['take']:32s} levels={levels!s:5s} edges in speech {row['edges_in_speech']}/{row['edges']}  "
                      + "  ".join(f"{float(k) * 1000:.0f}ms:{v['median']:.1f}/{v['max']:.1f}" for k, v in row["clicks"].items() if v["median"] is not None))
    if a.out:
        Path(a.out).write_text(json.dumps({"fades": FADES, "rows": rows}, indent=1))
    # The summary: edges in speech with and without the loudness, and the click per fade.
    for levels in (False, True):
        rs = [r for r in rows if r["levels"] == levels]
        print(f"levels={levels}: edges in speech {sum(r['edges_in_speech'] for r in rs)}/{sum(r['edges'] for r in rs)}")
        for fade in FADES:
            med = [r["clicks"][str(fade)]["median"] for r in rs if r["clicks"][str(fade)]["median"] is not None]
            mx = [r["clicks"][str(fade)]["max"] for r in rs if r["clicks"][str(fade)]["max"] is not None]
            print(f"   fade {fade * 1000:4.0f} ms: click median {np.median(med):.2f}, worst {np.max(mx):.2f}")


def breaths(a):
    """The breath after each cue: how much of it plays, per planner, per way of hearing."""
    cuts = a.cut or [CUT]
    rows = []
    for heard_as in ("browser", "room"):
        rng = np.random.default_rng(18)
        for name, script in SCRIPTS.items():
            if not any(is_cue(text) for text, _ in script):
                continue
            for voice in VOICES:
                pcm, marks = make(a.voices / f"{voice}.onnx", script, rng, heard_as)
                heard = listen(pcm, a.heard / f"breath-{heard_as}-{name}-{voice}.json")
                db = loudness(pcm)
                sil, thr = silences(db)
                t = {"duration": len(pcm) / RATE, "words": heard["words"], "silences": sil,
                     "levels": {"step": 0.01, "db": [round(float(x), 1) for x in db]}}
                for cut in cuts:
                    p = plan(t, cut)
                    keep = p["keep"]
                    for m in marks:
                        # Only a breath after a cue that was cut: one Whisper didn't write isn't the planner's.
                        cue_cut = kept(keep, (m["out"][0] - 0.3, m["out"][0])) < 0.15
                        rows.append({"heard_as": heard_as, "take": f"{name}/{voice}", "cut": cut.name, "cue_cut": cue_cut,
                                     "out_played": round(kept(keep, m["out"]), 3),
                                     "in_cut": round((m["in"][1] - m["in"][0]) - kept(keep, m["in"]), 3)})
                    print(f"{heard_as:8s} {name + '/' + voice:40s} {cut.name:16s} "
                          + "  ".join(f"out {r['out_played']:.2f} in-cut {r['in_cut']:.2f}{'' if r['cue_cut'] else ' (cue kept)'}"
                                      for r in rows[-len(marks):]))
    if a.out:
        Path(a.out).write_text(json.dumps({"breath": {"out": "0.55 s, -30 dBFS peak", "in": "0.35 s, -70 dBFS peak"}, "rows": rows}, indent=1))
    for heard_as in ("browser", "room"):
        for cut in cuts:
            rs = [r for r in rows if r["heard_as"] == heard_as and r["cut"] == cut.name and r["cue_cut"]]
            played = [r["out_played"] for r in rs]
            print(f"{heard_as:8s} {cut.name:16s} {len(rs)} breaths after a cut cue: breath out still playing in "
                  f"{sum(p > 0.05 for p in played)} (median {np.median(played):.2f} s, worst {max(played):.2f} s); "
                  f"breath in cut in {sum(r['in_cut'] > 0.05 for r in rs)}")


if __name__ == "__main__":
    sys.exit(main())
