#!/usr/bin/env python3
"""Packs one voice's POC run (run.py --render) into the editor prototype's files:
data.json (per take: length, sound levels, words, the planned cuts, the script) and
<scenario>.mp4 (the take before editing; the prototype applies the edit list as it plays).

    python3 build.py <run dir>/<voice> <out dir>
"""
import json
import shutil
import subprocess
import sys
from pathlib import Path

import numpy as np

src, out = Path(sys.argv[1]), Path(sys.argv[2])
out.mkdir(parents=True, exist_ok=True)
scripts = {s["id"]: s for s in json.loads((Path(__file__).parents[1] / "poc/scenarios.json").read_text())["scenarios"]}
takes = []
for d in sorted(p for p in src.iterdir() if (p / "edit-list.json").exists()):
    pcm = subprocess.run(["ffmpeg", "-loglevel", "error", "-i", str(d / "take.wav"), "-ac", "1", "-ar", "8000", "-f", "s16le", "-"],
                         capture_output=True, check=True).stdout
    a = np.abs(np.frombuffer(pcm, np.int16).astype(np.float32))
    n = 600
    peaks = [float(c.max()) if len(c) else 0.0 for c in np.array_split(a, n)]
    top = max(peaks) or 1
    t = json.loads((d / "transcript.json").read_text())
    e = json.loads((d / "edit-list.json").read_text())["edit_list"]
    shutil.copy(d / "before.mp4", out / f"{d.name}.mp4")
    takes.append({"id": d.name, "about": scripts[d.name]["about"], "duration": t["duration"],
                  "peaks": [round(p / top, 3) for p in peaks], "words": t["words"],
                  "cuts": e["cuts"], "video": f"{d.name}.mp4"})
(out / "data.json").write_text(json.dumps({"voice": src.name, "takes": takes}))
print(f"{len(takes)} takes → {out}")
