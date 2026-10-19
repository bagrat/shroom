#!/usr/bin/env python3
"""Writes the editor's extra canvas boards into <root>/project/: EditorCuts, EditorStates, EditorAfterSave, PausedMarker and EditorCutEdges.

They draw what the "Editing (a proposal)" boards leave out (DECISIONS B195): Undo and
Redo, a cut put back, the video fetched from the space, saving, and a save that did not
go through, the final video made after a save and the question of the
original (B197), and the paused popover's marker row. The helpers are the voice-cut proposal's (PR #59), so the column reads as one."""

import math
import random
import sys
from pathlib import Path

BG, CARD, LINE, FIELD, BORDER = "#2b2f36", "#333944", "#434a56", "#262a31", "#4c566a"
FG, FG3, MUTED, DIM = "#eceff4", "#c3cad6", "#aeb6c4", "#7b8494"
ACCENT, RED, AMBER, MENU = "#88c0d0", "#e88790", "#ebcb8b", "#3b4252"
FONT = "-apple-system, BlinkMacSystemFont, system-ui, sans-serif"


def page(title, w, h, body):
    return f"""<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<title>{title}</title>
<script src="./support.js"></script>
</head>
<body>
<x-dc>
<helmet>
<style>
body{{margin:0}}
a{{color:#88c0d0}}a:hover{{color:#8fbcbb}}
</style>
</helmet>
<div style="width: {w}px; height: {h}px; box-sizing: border-box; background: #1f2228; display: flex; flex-direction: column; font-family: {FONT}; font-size: 13px; line-height: 1.45; color: {FG}">
{body}
</div>
</x-dc>
<script type="text/x-dc" data-dc-script data-props='{{"$preview":{{"width":{w},"height":{h}}}}}'>
class Component extends DCLogic {{
renderVals() {{
return {{}};
}}
}}
</script>
</body>
</html>
"""


def chrome(title):
    dots = "".join(f'<span style="width: 12px; height: 12px; border-radius: 6px; background: {BORDER}; display: block"></span>' for _ in range(3))
    return (f'<div style="height: 36px; padding: 0 14px; display: flex; flex-direction: row; align-items: center; gap: 8px; flex: none; position: relative">{dots}'
            f'<span style="position: absolute; left: 0; right: 0; text-align: center; color: {MUTED}; font-size: 12px; pointer-events: none">{title}</span></div>')


def bars(n, seed, quiet=()):
    """Deterministic speech-like levels in 0..1; `quiet` = (from, to) fractions held low."""
    rnd = random.Random(seed)
    out, level = [], 0.5
    for i in range(n):
        f = i / n
        if any(a <= f < b for a, b in quiet):
            out.append(0.04 + rnd.random() * 0.03)
            continue
        level = min(1, max(0.12, level + rnd.uniform(-0.25, 0.25)))
        syll = 0.55 + 0.45 * abs(math.sin(i * 0.9 + rnd.random()))
        out.append(level * syll)
    return out


def waveform(w, h, levels, colour=MUTED):
    step = w / len(levels)
    rects = []
    for i, v in enumerate(levels):
        bh = max(2, v * (h - 6))
        rects.append(f'<rect x="{i * step:.1f}" y="{(h - bh) / 2:.1f}" width="{max(1, step - 1.2):.1f}" height="{bh:.1f}" rx="0.8"></rect>')
    return f'<svg viewBox="0 0 {w} {h}" aria-hidden="true" style="width: {w}px; height: {h}px; display: block; fill: {colour}">{"".join(rects)}</svg>'


def scissors(colour, size=14):
    return (f'<svg viewBox="0 0 16 16" aria-hidden="true" style="width: {size}px; height: {size}px; fill: none; stroke: {colour}; stroke-width: 1.4; stroke-linecap: round; display: block; flex: none">'
            '<circle cx="4.5" cy="4.5" r="2"></circle><circle cx="4.5" cy="11.5" r="2"></circle><path d="M6.2 5.6L13 11.8M6.2 10.4L13 4.2"></path></svg>')


def flag(colour):
    return f'<svg viewBox="0 0 10 12" aria-hidden="true" style="width: 10px; height: 12px; display: block; fill: {colour}"><path d="M1 0v12M1 1h8l-2 3 2 3H1z" stroke="{colour}" stroke-width="1.2"></path></svg>'


def button(label, primary=False, colour=None, said=None):
    s = f' data-said="{said}"' if said else ""
    if primary:
        return f'<button style="font: inherit; font-weight: 600; height: 30px; padding: 0 16px; border: 0; border-radius: 7px; background: {ACCENT}; color: {BG}; flex: none"{s}>{label}</button>'
    return f'<button style="font: inherit; height: 30px; padding: 0 14px; border: 1px solid {BORDER}; border-radius: 7px; background: transparent; color: {colour or FG}; flex: none"{s}>{label}</button>'


def preview(w, h, caption):
    """A recording's frame: a screen with a window on it and the camera bubble."""
    return f'''<div style="width: {w}px; height: {h}px; border-radius: 8px; background: #3b4252; position: relative; overflow: hidden; flex: none">
<div style="position: absolute; left: {w * 0.12:.0f}px; top: {h * 0.1:.0f}px; width: {w * 0.7:.0f}px; height: {h * 0.66:.0f}px; border-radius: 8px; background: #e5e9f0; display: flex; flex-direction: column">
<div style="height: 22px; border-bottom: 1px solid #d8dee9; display: flex; flex-direction: row; align-items: center; gap: 6px; padding: 0 10px"><span style="width: 8px; height: 8px; border-radius: 4px; background: #bf616a; display: block"></span><span style="width: 8px; height: 8px; border-radius: 4px; background: #ebcb8b; display: block"></span><span style="width: 8px; height: 8px; border-radius: 4px; background: #a3be8c; display: block"></span></div>
<div style="flex: 1 1 0; padding: 14px 18px; display: flex; flex-direction: column; gap: 8px">
<span style="height: 12px; width: 46%; border-radius: 3px; background: #4c566a; display: block"></span>
<span style="height: 8px; width: 80%; border-radius: 3px; background: #c3cad6; display: block"></span>
<span style="height: 8px; width: 72%; border-radius: 3px; background: #c3cad6; display: block"></span>
<span style="height: 8px; width: 64%; border-radius: 3px; background: #c3cad6; display: block"></span>
<span style="height: 40px; width: 50%; border-radius: 5px; background: #88c0d0; display: block; margin-top: 8px"></span>
</div>
</div>
<div style="position: absolute; left: 22px; bottom: 22px; width: {h * 0.26:.0f}px; height: {h * 0.26:.0f}px; border-radius: 50%; background: #5e81ac; border: 3px solid #eceff4"></div>
<div style="position: absolute; left: 0; right: 0; bottom: 14px; text-align: center"><span style="background: rgba(46, 52, 64, 0.82); color: {FG}; padding: 4px 10px; border-radius: 5px; font-size: 14px">{caption}</span></div>
</div>'''


def transport(now, total, was=None):
    play = f'<svg viewBox="0 0 16 16" aria-hidden="true" style="width: 14px; height: 14px; fill: {FG}; display: block"><path d="M5 3.5v9l7.5-4.5z"></path></svg>'
    was_s = f'<span style="color: {DIM}">was {was}</span>' if was else ""
    return (f'<div style="display: flex; flex-direction: row; align-items: center; gap: 12px">'
            f'<button aria-label="Play" style="width: 34px; height: 34px; border-radius: 17px; border: 0; background: {CARD}; display: flex; align-items: center; justify-content: center">{play}</button>'
            f'<span style="font-variant-numeric: tabular-nums">{now} / {total}</span>{was_s}</div>')


def timeline(w, total_s, cuts=(), offered=(), trim=None, markers=(), playhead=None, select=None, seed=7, extra=()):
    """The timeline: ruler, waveform, cuts (red), offered cuts (amber), trim handles; `extra`
    is drawn over the rest, in the track's own coordinates."""
    x = lambda t: t / total_s * w
    quiet = [(a / total_s, b / total_s) for a, b in []]
    wav = waveform(w, 66, bars(int(w / 4), seed, quiet))
    ticks = []
    step = 30 if total_s > 120 else 10
    for t in range(0, int(total_s) + 1, step):
        ticks.append(f'<span style="position: absolute; left: {x(t):.0f}px; top: 0; height: 18px; border-left: 1px solid {LINE}; padding-left: 4px; color: {DIM}; font-size: 11px; font-variant-numeric: tabular-nums">{t // 60}:{t % 60:02d}</span>')
    layers = []
    if trim:
        a, b = trim
        layers.append(f'<div style="position: absolute; left: 0; width: {x(a):.0f}px; top: 0; bottom: 0; background: rgba(31, 34, 40, 0.72)"></div>')
        layers.append(f'<div style="position: absolute; left: {x(b):.0f}px; right: 0; top: 0; bottom: 0; background: rgba(31, 34, 40, 0.72)"></div>')
        for t, side in ((a, "left"), (b, "right")):
            r = "6px 0 0 6px" if side == "left" else "0 6px 6px 0"
            off = x(t) - (12 if side == "left" else 0)
            layers.append(f'<div style="position: absolute; left: {off:.0f}px; top: -2px; bottom: -2px; width: 12px; border-radius: {r}; background: {AMBER}; display: flex; align-items: center; justify-content: center"><span style="width: 2px; height: 22px; border-radius: 1px; background: {BG}; display: block"></span></div>')
        layers.append(f'<div style="position: absolute; left: {x(a):.0f}px; width: {x(b) - x(a):.0f}px; top: -2px; bottom: -2px; border-top: 2px solid {AMBER}; border-bottom: 2px solid {AMBER}; box-sizing: border-box"></div>')
    for a, b in cuts:
        layers.append(f'<div style="position: absolute; left: {x(a):.0f}px; width: {max(4, x(b) - x(a)):.0f}px; top: 0; bottom: 0; background: repeating-linear-gradient(135deg, rgba(232, 135, 144, 0.55) 0 4px, rgba(232, 135, 144, 0.22) 4px 8px); border-left: 2px solid {RED}; border-right: 2px solid {RED}; box-sizing: border-box"></div>')
        layers.append(f'<div style="position: absolute; left: {(x(a) + x(b)) / 2 - 10:.0f}px; top: -26px; width: 20px; height: 20px; border-radius: 10px; background: {RED}; display: flex; align-items: center; justify-content: center">{scissors(BG, 12)}</div>')
    for a, b in offered:
        layers.append(f'<div style="position: absolute; left: {x(a):.0f}px; width: {max(4, x(b) - x(a)):.0f}px; top: 0; bottom: 0; border: 2px dashed {AMBER}; border-radius: 3px; box-sizing: border-box; background: rgba(235, 203, 139, 0.10)"></div>')
    if select:
        a, b = select
        layers.append(f'<div style="position: absolute; left: {x(a):.0f}px; width: {x(b) - x(a):.0f}px; top: 0; bottom: 0; background: rgba(136, 192, 208, 0.28); border-left: 2px solid {ACCENT}; border-right: 2px solid {ACCENT}; box-sizing: border-box"></div>')
        layers.append(f'<div style="position: absolute; left: {(x(a) + x(b)) / 2 - 56:.0f}px; top: 76px; display: flex; flex-direction: row; gap: 6px; padding: 4px; border-radius: 8px; background: {MENU}; border: 1px solid rgba(255, 255, 255, 0.12); box-shadow: 0 8px 24px rgba(0, 0, 0, 0.35)">'
                      f'<button style="font: inherit; height: 26px; padding: 0 10px; border: 0; border-radius: 6px; background: {RED}; color: {BG}; font-weight: 600; display: flex; flex-direction: row; align-items: center; gap: 6px">{scissors(BG, 12)}Cut {b - a:.1f} s</button></div>')
    for t in markers:
        layers.append(f'<div style="position: absolute; left: {x(t) - 1:.0f}px; top: -16px; bottom: 0; border-left: 1px dashed {ACCENT}"><div style="position: absolute; left: 0; top: 0">{flag(ACCENT)}</div></div>')
    if playhead is not None:
        layers.append(f'<div style="position: absolute; left: {x(playhead):.0f}px; top: -8px; bottom: -8px; width: 2px; background: {FG}"><span style="position: absolute; left: -5px; top: -4px; width: 12px; height: 12px; border-radius: 6px; background: {FG}; display: block"></span></div>')
    layers.extend(extra)
    return (f'<div style="width: {w}px; display: flex; flex-direction: column; gap: 10px; flex: none">'
            f'<div style="position: relative; height: 18px">{"".join(ticks)}</div>'
            f'<div style="position: relative; height: 66px; border-radius: 6px; background: {FIELD}">{wav}{"".join(layers)}</div></div>')


def icon_button(label, path, enabled=True):
    colour = FG if enabled else DIM
    return (f'<button aria-label="{label}" title="{label}" style="width: 30px; height: 30px; border-radius: 7px; border: 1px solid {BORDER}; background: transparent; display: flex; align-items: center; justify-content: center; flex: none; opacity: {1 if enabled else 0.5}">'
            f'<svg viewBox="0 0 16 16" aria-hidden="true" style="width: 15px; height: 15px; fill: none; stroke: {colour}; stroke-width: 1.5; stroke-linecap: round; stroke-linejoin: round; display: block">{path}</svg></button>')


UNDO = '<path d="M5.5 4L2.5 7l3 3"></path><path d="M2.5 7h7a4 4 0 0 1 0 8H7"></path>'
REDO = '<path d="M10.5 4l3 3-3 3"></path><path d="M13.5 7h-7a4 4 0 0 0 0 8H9"></path>'


def put_back_chip(left, length):
    return (f'<div style="position: absolute; left: {left - 62:.0f}px; top: 76px; display: flex; flex-direction: row; gap: 6px; padding: 4px; border-radius: 8px; background: {MENU}; border: 1px solid rgba(255, 255, 255, 0.12); box-shadow: 0 8px 24px rgba(0, 0, 0, 0.35)">'
            f'<button style="font: inherit; height: 26px; padding: 0 10px; border: 1px solid {BORDER}; border-radius: 6px; background: transparent; color: {FG}; display: flex; flex-direction: row; align-items: center; gap: 6px" data-said="editor.put-back">'
            f'<svg viewBox="0 0 16 16" aria-hidden="true" style="width: 12px; height: 12px; fill: none; stroke: {FG}; stroke-width: 1.5; stroke-linecap: round; stroke-linejoin: round; display: block">{UNDO}</svg>Put back {length:.1f} s</button></div>')


def footer(line, done="Done", done_enabled=True, line_colour=MUTED, revert=True):
    done_b = button(done, primary=True) if done_enabled else (
        f'<button disabled style="font: inherit; font-weight: 600; height: 30px; padding: 0 16px; border: 0; border-radius: 7px; background: {ACCENT}; color: {BG}; opacity: 0.5; flex: none">{done}</button>')
    rev = button("Revert to original", colour=FG3) if revert else ""
    return (f'<div style="display: flex; flex-direction: row; align-items: center; gap: 10px">'
            f'<span style="color: {line_colour}; flex: 1 1 0">{line}</span>{rev}{done_b}</div>')


def board_cuts():
    w = 1100
    tl_w = w - 48
    x = lambda t: t / 272 * tl_w
    tl = timeline(tl_w, 272, cuts=((150, 162),), trim=(8, 241), markers=(96, 205), playhead=84, seed=3)
    tl = tl[: tl.rfind("</div></div>")] + put_back_chip((x(150) + x(162)) / 2, 12.0) + "</div></div>"
    body = chrome("Edit — Setting up shroom in two minutes") + f'''
<div style="flex: 1 1 0; background: {BG}; padding: 8px 24px 20px 24px; display: flex; flex-direction: column; gap: 16px">
<div style="display: flex; flex-direction: row; justify-content: center">{preview(640, 360, "…and that's where the link lands.")}</div>
<div style="display: flex; flex-direction: row; align-items: center; gap: 12px">{transport("1:16", "3:39", "4:32")}
<span style="flex: 1 1 0"></span>
<span style="color: {MUTED}">Drag the yellow ends to trim. Drag across the sound to cut a part.</span>
{icon_button("Undo", UNDO)}{icon_button("Redo", REDO, enabled=False)}</div>
<div style="padding-top: 26px">{tl}</div>
<div style="flex: 1 1 0"></div>
{footer("Your link stays the same. Viewers see the edit once it's saved.")}
</div>'''
    return page("shroom Editor, a cut and undo", w, 720, body)


def strip(title, inner, w=1052):
    return (f'<div style="display: flex; flex-direction: column; gap: 8px">'
            f'<span style="color: {DIM}; font-size: 12px">{title}</span>'
            f'<div style="width: {w}px; box-sizing: border-box; background: {BG}; border-radius: 10px; padding: 16px 24px; display: flex; flex-direction: column; gap: 14px">{inner}</div></div>')


def board_states():
    tl_w = 1004
    getting = f'''<div style="display: flex; flex-direction: row; align-items: center; gap: 12px">{transport("0:00", "4:32")}</div>
<div style="height: 66px; border-radius: 6px; background: {FIELD}; display: flex; flex-direction: column; align-items: center; justify-content: center; gap: 8px">
<span style="color: {MUTED}" data-said="editor.getting">Getting the video from your space…</span>
<div style="width: 240px; height: 4px; border-radius: 2px; background: {LINE}; overflow: hidden"><div style="width: 40%; height: 4px; background: {ACCENT}"></div></div></div>
{footer("Your link stays the same. Viewers see the edit once it's saved.", done_enabled=False)}'''
    no_video = f'''<div style="height: 66px; border-radius: 6px; background: {FIELD}; display: flex; flex-direction: row; align-items: center; justify-content: center; gap: 12px">
<span style="color: {MUTED}" data-said="editor.cant-get">Couldn't get the video. Check your connection, then try again.</span>{button("Try again")}</div>
{footer("Your link stays the same. Viewers see the edit once it's saved.", done="Close", revert=False)}'''
    saving = footer("Saving your edit…", done="Saving…", done_enabled=False)
    failed = footer("Couldn't save your edit. Check your connection, then press Done again.", line_colour=AMBER)
    refused = footer("This video changed since you opened it, so this edit can't be saved.", done="Close", revert=False, line_colour=AMBER)
    body = f'''<div style="flex: 1 1 0; padding: 24px; display: flex; flex-direction: column; gap: 20px">
{strip("When the video isn't on this Mac: it comes from your space first (the waveform waits for it; Done waits too)", getting)}
{strip("When it can't be fetched", no_video)}
{strip("Done, while the edit saves (the window closes when it's saved)", saving)}
{strip("Done, when the save didn't go through (the window stays open, nothing is lost)", failed)}
{strip("Done, when the space refused it: another Mac made the edit the video, or it was deleted (Close, Return or Esc closes; nothing is saved)", refused)}
</div>'''
    return page("shroom Editor, getting ready and saving", 1100, 820, body)


def board_after_save():
    """After Done: the final video is made, then the original's fate is asked (B197)."""
    bar = lambda pct: f'<div style="width: 240px; height: 4px; border-radius: 2px; background: {LINE}; overflow: hidden"><div style="width: {pct}%; height: 4px; background: {ACCENT}"></div></div>'
    making = f'''<div style="height: 66px; border-radius: 6px; background: {FIELD}; display: flex; flex-direction: column; align-items: center; justify-content: center; gap: 8px">
<span style="color: {FG}" data-said="editor-making.making">Making the final video… 40%</span>{bar(40)}</div>
{footer("The link already plays your edit. You can close this window: shroom finishes it.", done="Close", revert=False)}'''
    ask = f'''<div style="display: flex; flex-direction: row; align-items: center; gap: 16px; background: {CARD}; border: 1px solid {LINE}; border-radius: 10px; padding: 14px 16px">
<div style="flex: 1 1 0; display: flex; flex-direction: column; gap: 4px">
<span style="font-weight: 600; font-size: 14px" data-said="editor-keep-original.title">Delete the original?</span>
<span style="color: {MUTED}" data-said="editor-keep-original.line">Your edit is the video now. The original is only kept so you can revert to it.</span></div>
{button("Keep for 2 days", colour=FG3, said="editor-keep-original.keep")}{button("Yes, delete", primary=True, said="editor-keep-original.delete")}</div>'''
    failed = footer("Couldn't make the final video. The link plays your edit anyway, and shroom tries again later.", done="Close", revert=False, line_colour=AMBER)
    kept = (f'<div style="display: flex; flex-direction: row; align-items: center; gap: 10px">'
            f'<span style="color: {MUTED}; flex: 1 1 0">Your link stays the same. Viewers see the edit once it\'s saved.</span>'
            f'<span style="color: {DIM}; font-size: 12px" data-said="editor.original-kept">The original is kept one more day</span>'
            f'{button("Revert to original", colour=FG3)}{button("Done", primary=True)}</div>')
    gone = (f'<div style="display: flex; flex-direction: row; align-items: center; gap: 10px">'
            f'<span style="color: {MUTED}; flex: 1 1 0">Your link stays the same. Viewers see the edit once it\'s saved.</span>'
            f'<button disabled style="font: inherit; height: 30px; padding: 0 14px; border: 1px solid {BORDER}; border-radius: 7px; background: transparent; color: {DIM}; flex: none; opacity: 0.6">Revert to original</button>{button("Done", primary=True)}</div>')
    body = f'''<div style="flex: 1 1 0; padding: 24px; display: flex; flex-direction: column; gap: 20px">
{strip("Saved: the final video is made on this Mac and put up (the link already plays the edit)", making)}
{strip("Once it's up: the window asks, Yes, delete leading (a click: Return does nothing here). ✕, Esc or Close means Keep for 2 days", ask)}
{strip("When it couldn't be made (offline, the space didn't answer)", failed)}
{strip("Editing again while the original is kept: the time it has left sits by Revert", kept)}
{strip("Once the original is gone the edit is final: Revert to original only undoes what's changed since the window opened", gone)}
</div>'''
    return page("shroom Editor, after saving", 1100, 760, body)


def board_paused_marker():
    sq = lambda path: f'<svg viewBox="0 0 16 16" aria-hidden="true" style="width: 16px; height: 16px; fill: none; stroke: {MUTED}; stroke-width: 1.4; stroke-linecap: round; stroke-linejoin: round; display: block; flex: none">{path}</svg>'
    stop = sq('<rect x="3.5" y="3.5" width="9" height="9" rx="2"></rect>')
    restart = sq('<path d="M3 8a5 5 0 1 0 1.6-3.7M3 2.5v2.5h2.5"></path>')
    trash = f'<svg viewBox="0 0 16 16" aria-hidden="true" style="width: 16px; height: 16px; fill: none; stroke: {RED}; stroke-width: 1.4; stroke-linecap: round; stroke-linejoin: round; display: block; flex: none"><path d="M3 4.5h10M6.5 4.5V3h3v1.5M4.5 4.5l.6 8.5h5.8l.6-8.5"></path></svg>'
    check = f'<svg viewBox="0 0 16 16" aria-hidden="true" style="width: 16px; height: 16px; fill: none; stroke: {DIM}; stroke-width: 1.6; stroke-linecap: round; stroke-linejoin: round; display: block; flex: none"><path d="M3.5 8.5l3 3 6-7"></path></svg>'

    def row(icon, label, colour=FG, top=True, said=None, enabled=True):
        s = f' data-said="{said}"' if said else ""
        dis = "" if enabled else " disabled"
        return (f'<button{dis} style="font: inherit; height: 36px; padding: 0 10px; border: 0; {"border-top: 1px solid " + LINE + ";" if top else ""} background: transparent; color: {colour if enabled else DIM}; display: flex; flex-direction: row; align-items: center; gap: 10px; text-align: left">'
                f'{icon}<span{s}>{label}</span></button>')

    def popover(dropped):
        marker = (row(check, "Marker dropped", said="paused.marker-dropped", enabled=False) if dropped
                  else row(f'<span style="width: 16px; height: 16px; display: flex; align-items: center; justify-content: center; flex: none">{flag(ACCENT)}</span>', "Drop a marker", said="paused.drop-marker"))
        return f'''<div style="width: 300px; box-sizing: border-box; background: {BG}; border: 1px solid rgba(255, 255, 255, 0.1); border-radius: 12px; box-shadow: 0 8px 28px rgba(0, 0, 0, 0.3); padding: 14px; display: flex; flex-direction: column; gap: 12px">
<h1 style="margin: 0; font-size: 15px; font-weight: 600; line-height: 28px" data-said="paused.paused-4-32">Paused — 4:32</h1>
<button style="font: inherit; font-weight: 600; height: 36px; border: 0; border-radius: 8px; background: {ACCENT}; color: {BG}" data-said="paused.resume">Resume</button>
<div style="background: {CARD}; border-radius: 9px; display: flex; flex-direction: column">
{row(stop, "Finish &amp; Share", top=False, said="paused.stop")}{marker}{row(restart, "Restart", said="paused.restart")}{row(trash, "Discard…", colour=RED, said="paused.discard")}
</div></div>'''
    body = f'''<div style="flex: 1 1 0; padding: 28px; display: flex; flex-direction: row; gap: 48px; align-items: flex-start">
{mini("Paused: Drop a marker, between Finish &amp; Share and Restart (N64)", popover(False), 300)}
{mini("Once dropped at this pause: greyed until Resume", popover(True), 300)}
{mini("While recording, ⌥⌘M drops one at that moment: nothing on screen answers", f'<span style="color: {MUTED}">The take goes on. The flag shows on the editor\'s timeline.</span>', 300)}
</div>'''
    return page("shroom Popover, paused, a marker dropped", 1100, 360, body)


def edge_handle(left, colour=RED):
    """A cut's edge under the pointer: a wider bar with a grip, the full height of the sound."""
    return (f'<div style="position: absolute; left: {left - 4:.0f}px; top: -6px; bottom: -6px; width: 8px; border-radius: 4px; background: {colour}; box-shadow: 0 0 0 2px rgba(31, 34, 40, 0.9); display: flex; flex-direction: column; align-items: center; justify-content: center; gap: 3px">'
            + "".join(f'<span style="width: 2px; height: 2px; border-radius: 1px; background: {BG}; display: block"></span>' for _ in range(3)) + '</div>')


def resize_cursor(left, top=24):
    """macOS's resize pointer (↔ with a bar), drawn where the pointer is."""
    path = '<path d="M11 2v18"></path><path d="M7 11H2m0 0l3-3m-3 3l3 3M15 11h5m0 0l-3-3m3 3l-3 3"></path>'
    return (f'<svg viewBox="0 0 22 22" aria-hidden="true" style="position: absolute; left: {left - 11:.0f}px; top: {top}px; width: 22px; height: 22px; overflow: visible; fill: none; stroke-linecap: round; stroke-linejoin: round">'
            f'<g stroke="#1f2228" stroke-width="4">{path}</g><g stroke="{FG}" stroke-width="1.6">{path}</g></svg>')


def ghost(left):
    """Where the edge was when the drag began."""
    return f'<div style="position: absolute; left: {left - 1:.0f}px; top: 0; bottom: 0; border-left: 2px dotted {FG3}; opacity: 0.8"></div>'


def chip_at(centre, label, colour=RED, ink=BG, icon=True):
    """The chip under what's selected or dragged."""
    sc = scissors(ink, 12) if icon else ""
    return (f'<div style="position: absolute; left: {centre - 58:.0f}px; top: 76px; display: flex; flex-direction: row; gap: 6px; padding: 4px; border-radius: 8px; background: {MENU}; border: 1px solid rgba(255, 255, 255, 0.12); box-shadow: 0 8px 24px rgba(0, 0, 0, 0.35)">'
            f'<button style="font: inherit; height: 26px; padding: 0 10px; border: 0; border-radius: 6px; background: {colour}; color: {ink}; font-weight: 600; display: flex; flex-direction: row; align-items: center; gap: 6px; white-space: nowrap">{sc}{label}</button></div>')


def board_cut_edges():
    w, total = 1004, 60
    x = lambda t: t / total * w
    cuts, trim = [(20, 28), (36, 40)], (3, 55)
    tl = lambda width=w, **k: timeline(width, total, trim=trim, seed=11, **{"cuts": cuts, **k})
    hover = tl(extra=[edge_handle(x(28)), resize_cursor(x(28) + 18)])
    # Dragging the right edge out to 30.4 s: the cut grows, the chip says how long it is now, the
    # picture shows the frame at the edge.
    wd = 760
    xd = lambda t: t / total * wd
    drag = timeline(wd, total, cuts=[(20, 30.4), (36, 40)], trim=trim, seed=11,
                    extra=[ghost(xd(28)), edge_handle(xd(30.4)), resize_cursor(xd(30.4) + 14), chip_at(xd(25.2), "Cut 10.4 s")])
    frame = (f'<div style="display: flex; flex-direction: column; gap: 6px; flex: none">{preview(214, 120, "0:30.4")}'
             f'<span style="color: {DIM}; font-size: 12px">the picture follows the edge</span></div>')
    # It stops: flush against the next cut, and at the trim's end.
    wh = 470
    xh = lambda t: t / total * wh
    at_cut = timeline(wh, total, cuts=[(20, 36), (36, 40)], trim=trim, seed=11, extra=[edge_handle(xh(36)), resize_cursor(xh(36) + 14)])
    at_end = timeline(wh, total, cuts=[(20, 28), (36, 55)], trim=trim, seed=11, extra=[edge_handle(xh(55)), resize_cursor(xh(55) - 18)])
    stops = (f'<div style="display: flex; flex-direction: row; gap: 64px">'
             f'<div style="display: flex; flex-direction: column; gap: 8px">{at_cut}<span style="color: {DIM}; font-size: 12px; padding-top: 4px">at the next cut: they touch and stay two cuts</span></div>'
             f'<div style="display: flex; flex-direction: column; gap: 8px">{at_end}<span style="color: {DIM}; font-size: 12px; padding-top: 4px">at the yellow end of the trim</span></div></div>')
    # Let go: one step. The cut stays picked, Put back says its new length; Undo puts the edge back.
    done = tl(cuts=[(20, 30.4), (36, 40)], extra=[])
    done = done[: done.rfind("</div></div>")] + put_back_chip((x(20) + x(30.4)) / 2, 10.4) + "</div></div>"
    undo_row = (f'<div style="display: flex; flex-direction: row; align-items: center; gap: 10px; justify-content: flex-end">'
                f'<span style="color: {DIM}; font-size: 12px">Undo (⌘Z) puts the edge back where it was; Redo moves it again</span>{icon_button("Undo", UNDO)}{icon_button("Redo", REDO, enabled=False)}</div>')
    # A suggestion (dashed amber) the same way: its edges move, it stays offered, its Cut chip takes it as it now is.
    offered = timeline(w, total, cuts=cuts, offered=[(42.5, 49)], trim=trim, seed=11,
                       extra=[ghost(x(44)), edge_handle(x(42.5), AMBER), resize_cursor(x(42.5) - 18), chip_at((x(42.5) + x(49)) / 2, "Cut 6.5 s")])
    body = f'''<div style="flex: 1 1 0; padding: 24px; display: flex; flex-direction: column; gap: 22px">
{strip("A cut's edge under the pointer: the pointer becomes ↔ and the edge stands out (either edge of any red cut)", f'<div style="padding-top: 26px">{hover}</div><div style="height: 18px"></div>')}
{strip("Dragging it: the cut grows or shrinks as it goes, its chip says the length it has now, and the picture shows the frame at the edge", f'<div style="display: flex; flex-direction: row; gap: 30px; align-items: flex-start">{frame}<div style="padding-top: 26px">{drag}</div></div><div style="height: 22px"></div>')}
{strip("It stops where a part already goes: at the next cut, the end of the trim, a tenth of a second from its other edge, or where a second would be all that's left", f'<div style="padding-top: 26px">{stops}</div>')}
{strip("Let go: one step, like any other. The cut stays picked, and Put back says its new length", f'<div style="padding-top: 26px">{done}</div><div style="height: 30px"></div>{undo_row}')}
{strip("A suggestion (dashed) moves the same way: it stays a suggestion, and its chip (Cut and its length, as under a selection) takes it as it now is", f'<div style="padding-top: 26px">{offered}</div><div style="height: 30px"></div>')}
</div>'''
    return page("shroom Editor, adjusting a cut", 1100, 1290, body)


def mini(title, inner, w):
    return (f'<div style="width: {w}px; display: flex; flex-direction: column; gap: 10px; flex: none">'
            f'<span style="color: {MUTED}; font-size: 12px">{title}</span>{inner}</div>')


def main():
    root = Path(sys.argv[1]) / "project"
    root.mkdir(parents=True, exist_ok=True)
    (root / "EditorCuts.dc.html").write_text(board_cuts())
    (root / "EditorStates.dc.html").write_text(board_states())
    (root / "PausedMarker.dc.html").write_text(board_paused_marker())
    (root / "EditorAfterSave.dc.html").write_text(board_after_save())
    (root / "EditorCutEdges.dc.html").write_text(board_cut_edges())


if __name__ == "__main__":
    main()
