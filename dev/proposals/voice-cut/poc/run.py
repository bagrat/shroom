#!/usr/bin/env python3
"""Voice-cue POC harness (dev/proposals/voice-cut.md).

For each invented script: speak it with a stock TTS voice (never a person's clone),
transcribe it with Whisper (word times), find silences, run the planner
(crates/shroom-edit, the `shroom-cut` binary), score the cuts against the script, and
render a before and an after video plus the edit list.

    pip install piper-tts faster-whisper pillow
    python3 -m piper.download_voices en_US-lessac-medium   # once per voice, into --voices
    cargo build -p shroom-edit --release
    python3 dev/proposals/voice-cut/poc/run.py --voices DIR --out DIR [--render] [--model small.en]

A real recording instead of a script: --wav take.wav (no scoring, same outputs).
"""

import argparse
import json
import re
import subprocess
import sys
import wave
from pathlib import Path

import numpy as np

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[3]
SR_OUT = 22050
LEAD = 0.6  # silence before the first line
PROMPT = "Shroom cut. Um, uh, so, so I, I mean, shroom cut."


def synth(voice, lines):
    """Speak each line, a pause after it; return audio and each line's [start, end)."""
    chunks, spans, t = [np.zeros(int(LEAD * SR_OUT), np.int16)], [], LEAD
    for line in lines:
        audio = np.concatenate([c.audio_int16_array for c in voice.synthesize(line["say"])])
        audio = trim_silence(audio)
        spans.append((t, t + len(audio) / SR_OUT))
        t += len(audio) / SR_OUT + line["pause"]
        chunks += [audio, np.zeros(int(line["pause"] * SR_OUT), np.int16)]
    return np.concatenate(chunks), spans


def trim_silence(a, thresh=300):
    loud = np.nonzero(np.abs(a) > thresh)[0]
    return a[max(loud[0] - 220, 0): loud[-1] + 220] if len(loud) else a


def write_wav(path, audio):
    with wave.open(str(path), "wb") as w:
        w.setnchannels(1)
        w.setsampwidth(2)
        w.setframerate(SR_OUT)
        w.writeframes(audio.tobytes())


def silences(wav):
    err = subprocess.run(
        ["ffmpeg", "-hide_banner", "-i", str(wav), "-af", "silencedetect=noise=-38dB:d=0.12", "-f", "null", "-"],
        capture_output=True, text=True).stderr
    starts = [float(x) for x in re.findall(r"silence_start: ([\d.]+)", err)]
    ends = [float(x) for x in re.findall(r"silence_end: ([\d.]+)", err)]
    return [{"start": s, "end": e} for s, e in zip(starts, ends)]


def duration(wav):
    with wave.open(str(wav)) as w:
        return w.getnframes() / w.getframerate()


def transcribe(model, wav, prompt):
    pcm = subprocess.run(["ffmpeg", "-loglevel", "error", "-i", str(wav), "-ac", "1", "-ar", "16000", "-f", "f32le", "-"],
                         capture_output=True, check=True).stdout
    segs, _ = model.transcribe(np.frombuffer(pcm, np.float32), language="en", word_timestamps=True, initial_prompt=prompt,
                               condition_on_previous_text=False, beam_size=5)
    return [{"text": w.word.strip(), "start": round(w.start, 3), "end": round(w.end, 3)}
            for s in segs for w in (s.words or [])]


def plan(transcript):
    exe = ROOT / "target/release/shroom-cut"
    out = subprocess.run([str(exe)], input=json.dumps(transcript), capture_output=True, text=True, check=True)
    return json.loads(out.stdout)


def overlap(a, b):
    return max(0.0, min(a[1], b[1]) - max(a[0], b[0]))


def score(lines, spans, cuts):
    """Seconds of each role's speech removed by applied cuts, and covered by suggestions."""
    applied = [(c["start"], c["end"]) for c in cuts if c["applied"]]
    offered = [(c["start"], c["end"]) for c in cuts if not c["applied"]]
    s = {}
    for line, span in zip(lines, spans):
        r = s.setdefault(line["role"], {"speech": 0.0, "cut": 0.0, "offered": 0.0})
        r["speech"] += span[1] - span[0]
        r["cut"] += sum(overlap(span, c) for c in applied)
        r["offered"] += sum(overlap(span, c) for c in offered)
    return {k: {kk: round(vv, 2) for kk, vv in v.items()} for k, v in s.items()}


def verdict(sc, scenario_id):
    """Pass when every cue goes, no kept or mentioned speech is cut, and every flub goes
    (or, for a cue with no retake, is offered)."""
    bad = sum(sc.get(r, {}).get("cut", 0) for r in ("keep", "mention"))
    cue = sc.get("cue")
    flub = sc.get("flub")
    ok = bad < 0.08
    if cue:
        ok &= cue["cut"] >= cue["speech"] * 0.9
    if flub:
        gone = flub["cut"] + flub["offered"]
        ok &= gone >= flub["speech"] * 0.9
    return ok


def ass(lines, spans, path):
    colour = {"keep": "&H00F2F2F2", "flub": "&H006B6BFF", "cue": "&H0040B8FF", "mention": "&H00F2F2F2"}
    head = ("[Script Info]\nScriptType: v4.00+\nPlayResX: 1280\nPlayResY: 720\n\n[V4+ Styles]\n"
            "Format: Name, Fontname, Fontsize, PrimaryColour, OutlineColour, BorderStyle, Outline, Shadow, Alignment, MarginL, MarginR, MarginV\n"
            "Style: L,Inter,40,&H00F2F2F2,&H00000000,1,0,0,2,90,90,120\n\n[Events]\n"
            "Format: Layer, Start, End, Style, Text\n")

    def ts(t):
        return f"{int(t // 3600)}:{int(t % 3600 // 60):02d}:{t % 60:05.2f}"

    rows = [f"Dialogue: 0,{ts(a)},{ts(b + 0.25)},L,{{\\c{colour[l['role']]}}}{l['say']}" for l, (a, b) in zip(lines, spans)]
    path.write_text(head + "\n".join(rows) + "\n")


def render(wav, out_dir, dur, keep, subs=None):
    font = "/usr/share/fonts/opentype/inter/Inter-Medium.otf"
    vf = [f"drawtext=fontfile={font}:text='source %{{pts\\:hms}}':x=60:y=50:fontsize=34:fontcolor=0xC9A27A"]
    if subs:
        vf.insert(0, f"subtitles={subs}")
    before = out_dir / "before.mp4"
    subprocess.run(["ffmpeg", "-y", "-loglevel", "error", "-f", "lavfi", "-i", f"color=c=0x1d1a17:s=1280x720:r=30:d={dur:.3f}",
                    "-i", str(wav), "-vf", ",".join(vf), "-c:v", "libx264", "-preset", "veryfast", "-crf", "26",
                    "-c:a", "aac", "-b:a", "128k", "-shortest", str(before)], check=True)
    parts, labels = [], []
    for i, (s, e) in enumerate(keep):
        f = min(0.008, (e - s) / 4)
        parts.append(f"[0:v]trim={s:.3f}:{e:.3f},setpts=PTS-STARTPTS[v{i}]")
        parts.append(f"[0:a]atrim={s:.3f}:{e:.3f},asetpts=PTS-STARTPTS,afade=t=in:d={f:.3f},afade=t=out:st={e - s - f:.3f}:d={f:.3f}[a{i}]")
        labels.append(f"[v{i}][a{i}]")
    parts.append("".join(labels) + f"concat=n={len(keep)}:v=1:a=1[v][a]")
    subprocess.run(["ffmpeg", "-y", "-loglevel", "error", "-i", str(before), "-filter_complex", ";".join(parts),
                    "-map", "[v]", "-map", "[a]", "-c:v", "libx264", "-preset", "veryfast", "-crf", "26",
                    "-c:a", "aac", "-b:a", "128k", str(out_dir / "after.mp4")], check=True)


def strip(wav, out_png, dur, words, cuts, spans=None, lines=None):
    """A picture of the take: waveform, words, applied cuts (red) and offered ones (amber)."""
    from PIL import Image, ImageDraw, ImageFont
    w, h = 1600, 300
    tmp = out_png.with_suffix(".wave.png")
    subprocess.run(["ffmpeg", "-y", "-loglevel", "error", "-i", str(wav), "-filter_complex",
                    f"showwavespic=s={w}x160:colors=0xC9A27A", "-frames:v", "1", str(tmp)], check=True)
    img = Image.new("RGB", (w, h), (29, 26, 23))
    img.paste(Image.open(tmp).convert("RGB"), (0, 60))
    tmp.unlink()
    d = ImageDraw.Draw(img, "RGBA")
    font = ImageFont.truetype("/usr/share/fonts/opentype/inter/Inter-Regular.otf", 13)
    x = lambda t: int(t / dur * w)
    if spans:
        tint = {"flub": (255, 107, 107, 50), "cue": (255, 184, 64, 60)}
        for l, (a, b) in zip(lines, spans):
            if l["role"] in tint:
                d.rectangle([x(a), 40, x(b), 52], fill=tint[l["role"]][:3] + (200,))
    for c in cuts:
        box = [x(c["start"]), 58, x(c["end"]), 222]
        if c["applied"]:
            d.rectangle(box, fill=(220, 60, 60, 90), outline=(240, 80, 80, 255))
        else:
            d.rectangle(box, outline=(255, 184, 64, 255), width=2)
    row = 0
    for i, wd in enumerate(words):
        d.text((x(wd["start"]), 232 + 18 * (i % 3)), wd["text"], font=font, fill=(230, 225, 220))
    d.text((8, 8), "script: red = flub, amber = cue  ·  cuts: red fill = applied, amber outline = offered", font=font, fill=(200, 190, 180))
    img.save(out_png)


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--voices", type=Path, required=True)
    ap.add_argument("--out", type=Path, required=True)
    ap.add_argument("--model", default="small.en")
    ap.add_argument("--voice", action="append", help="voice names (default: every .onnx in --voices)")
    ap.add_argument("--only", help="scenario id prefix")
    ap.add_argument("--no-prompt", action="store_true")
    ap.add_argument("--render", action="store_true")
    ap.add_argument("--wav", type=Path, help="a real recording instead of the scripts")
    a = ap.parse_args()

    from faster_whisper import WhisperModel
    model = WhisperModel(a.model, device="cpu", compute_type="int8")
    prompt = None if a.no_prompt else PROMPT
    a.out.mkdir(parents=True, exist_ok=True)

    if a.wav:
        d = a.out / a.wav.stem
        d.mkdir(exist_ok=True)
        wav = d / "take.wav"
        subprocess.run(["ffmpeg", "-y", "-loglevel", "error", "-i", str(a.wav), "-ac", "1", "-ar", str(SR_OUT), str(wav)], check=True)
        dur = duration(wav)
        t = {"duration": dur, "words": transcribe(model, wav, prompt), "silences": silences(wav)}
        p = plan(t)
        (d / "transcript.json").write_text(json.dumps(t, indent=1))
        (d / "edit-list.json").write_text(json.dumps(p, indent=1))
        strip(wav, d / "strip.png", dur, t["words"], p["edit_list"]["cuts"])
        if a.render:
            render(wav, d, dur, p["keep"])
        print(json.dumps(p["edit_list"]["cuts"], indent=1))
        return

    from piper import PiperVoice
    scenarios = json.loads((HERE / "scenarios.json").read_text())["scenarios"]
    names = a.voice or sorted(p.stem for p in a.voices.glob("*.onnx"))
    results = []
    for name in names:
        voice = PiperVoice.load(str(a.voices / f"{name}.onnx"))
        for sc in scenarios:
            if a.only and not sc["id"].startswith(a.only):
                continue
            d = a.out / name / sc["id"]
            d.mkdir(parents=True, exist_ok=True)
            audio, spans = synth(voice, sc["lines"])
            wav = d / "take.wav"
            write_wav(wav, audio)
            dur = duration(wav)
            t = {"duration": dur, "words": transcribe(model, wav, prompt), "silences": silences(wav)}
            p = plan(t)
            cuts = p["edit_list"]["cuts"]
            sc_score = score(sc["lines"], spans, cuts)
            ok = verdict(sc_score, sc["id"])
            (d / "transcript.json").write_text(json.dumps(t, indent=1))
            (d / "edit-list.json").write_text(json.dumps(p, indent=1))
            (d / "script.json").write_text(json.dumps([{**l, "start": round(a, 3), "end": round(b, 3)} for l, (a, b) in zip(sc["lines"], spans)], indent=1))
            strip(wav, d / "strip.png", dur, t["words"], cuts, spans, sc["lines"])
            if a.render:
                subs = d / "lines.ass"
                ass(sc["lines"], spans, subs)
                render(wav, d, dur, p["keep"], subs)
            heard = " ".join(w["text"] for w in t["words"])
            results.append({"voice": name, "scenario": sc["id"], "pass": ok, "score": sc_score,
                            "cuts": [{k: c[k] for k in ("start", "end", "kind", "confidence", "applied", "removed")} for c in cuts],
                            "heard": heard, "before": round(dur, 2), "after": round(p["edited_duration"], 2)})
            print(f"{'PASS' if ok else 'FAIL'}  {name:22s} {sc['id']:20s} {dur:5.1f}s → {p['edited_duration']:5.1f}s  | {heard}", flush=True)
            for c in cuts:
                print(f"        {'✂' if c['applied'] else '?'} {c['start']:6.2f}–{c['end']:6.2f} {c['kind']:16s} “{c['removed']}”")
    (a.out / "results.json").write_text(json.dumps(results, indent=1))
    n = sum(r["pass"] for r in results)
    print(f"\n{n}/{len(results)} pass")


if __name__ == "__main__":
    sys.exit(main())
