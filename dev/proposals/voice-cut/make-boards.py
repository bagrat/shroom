#!/usr/bin/env python3
"""Writes the editing proposal's canvas boards (dev/proposals/voice-cut.md) into
<root>/project/: EditorTrim, EditorVoiceCuts, EditorDoors, and the voice-cut build's
owner-page boards (spec N65): OwnerPageCuts and OwnerPageCutRows. Words are provisional;
the look follows the canvas's dark Nord boards (app/src/lib/theme.css) and, for the owner
page, its OwnerPageLive board."""

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


def timeline(w, total_s, cuts=(), offered=(), trim=None, markers=(), playhead=None, select=None, seed=7):
    """The timeline: ruler, waveform, cuts (red), offered cuts (amber), trim handles."""
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
    return (f'<div style="width: {w}px; display: flex; flex-direction: column; gap: 10px; flex: none">'
            f'<div style="position: relative; height: 18px">{"".join(ticks)}</div>'
            f'<div style="position: relative; height: 66px; border-radius: 6px; background: {FIELD}">{wav}{"".join(layers)}</div></div>')


def board_trim():
    w = 1100
    body = chrome("Edit — Setting up shroom in two minutes") + f'''
<div style="flex: 1 1 0; background: {BG}; padding: 8px 24px 20px 24px; display: flex; flex-direction: column; gap: 16px">
<div style="display: flex; flex-direction: row; justify-content: center">{preview(640, 360, "…and that's where the link lands.")}</div>
<div style="display: flex; flex-direction: row; align-items: center; justify-content: space-between">{transport("0:08", "3:51", "4:32")}
<span style="color: {MUTED}">Drag the yellow ends to trim. Drag across the sound to cut a part.</span></div>
<div style="padding-top: 26px">{timeline(w - 48, 272, trim=(8, 241), markers=(96,), playhead=8, select=(150, 162), seed=3)}</div>
<div style="flex: 1 1 0"></div>
<div style="display: flex; flex-direction: row; align-items: center; gap: 10px">
<span style="color: {MUTED}; flex: 1 1 0">Your link stays the same. Viewers see the edit once it's saved.</span>
{button("Revert to original", colour=FG3)}{button("Done", primary=True)}</div>
</div>'''
    return page("shroom Editor, trim", w, 720, body)


def cut_card(when, length, gone, then, applied=True, note=None, action="Undo"):
    colour = RED if applied else AMBER
    edge = f"border: 1px solid {LINE}" if applied else f"border: 1px dashed {AMBER}"
    note_s = f'<span style="color: {AMBER}; font-size: 12px">{note}</span>' if note else ""
    gone_style = "text-decoration: line-through; text-decoration-color: rgba(232, 135, 144, 0.9); color: " + (FG3 if applied else MUTED)
    return f'''<div style="background: {CARD}; {edge}; border-radius: 9px; padding: 10px 12px; display: flex; flex-direction: column; gap: 6px">
<div style="display: flex; flex-direction: row; align-items: center; gap: 8px">{scissors(colour)}<span style="font-weight: 600; font-variant-numeric: tabular-nums">{when}</span><span style="color: {DIM}">· {length}</span><span style="flex: 1 1 0"></span>
<button style="font: inherit; border: 0; padding: 0 4px; background: transparent; color: {FG3}">Play</button>
<button style="font: inherit; height: 26px; padding: 0 10px; border: 1px solid {BORDER}; border-radius: 6px; background: transparent; color: {FG}">{action}</button></div>
{note_s}<span style="{gone_style}">“{gone}”</span>
<span style="color: {MUTED}; font-size: 12px">then: “{then}”</span></div>'''


def board_cues():
    w, side = 1320, 380
    left = w - side - 48 - 20
    cards = "".join([
        cut_card("0:04", "3.4 s", "Hello, that is shroom. Shroom cut.", "Hello, this is shroom."),
        cut_card("1:12", "4.1 s", "Then you pick the, the window you, um, shroom cut.", "So, then you pick the window you want to share."),
        cut_card("2:47", "1.9 s", "The second one is, um, I forget what it's called. Shroom cut.", "Let's move on to the Library instead.", applied=False,
                 note="Nothing was said again after this one, so only “shroom cut” went. The line before it too?", action="Cut it too"),
    ])
    body = chrome("Edit — Setting up shroom in two minutes") + f'''
<div style="flex: 1 1 0; background: {BG}; padding: 8px 24px 20px 24px; display: flex; flex-direction: column; gap: 16px">
<div style="display: flex; flex-direction: row; gap: 20px; min-height: 0">
<div style="width: {left}px; display: flex; flex-direction: column; gap: 14px">
{preview(left, round(left * 9 / 16), "Hello, this is shroom.")}
{transport("0:04", "3:58", "4:32")}
</div>
<div style="width: {side}px; display: flex; flex-direction: column; gap: 10px">
<div style="display: flex; flex-direction: column"><h1 style="margin: 0; font-size: 16px; font-weight: 600">Cuts you asked for</h1>
<span style="color: {MUTED}">You said “shroom cut” 3 times. Two slips are gone; one is for you to look at.</span></div>
{cards}
<div style="background: {CARD}; border: 1px solid {LINE}; border-radius: 9px; padding: 10px 12px; display: flex; flex-direction: row; align-items: center; gap: 8px">{scissors(RED)}<span style="font-weight: 600">1:52</span><span style="color: {DIM}">· 6.0 s · cut by you</span><span style="flex: 1 1 0"></span>
<button style="font: inherit; height: 26px; padding: 0 10px; border: 1px solid {BORDER}; border-radius: 6px; background: transparent; color: {FG}">Undo</button></div>
</div>
</div>
<div style="padding-top: 28px">{timeline(w - 48, 272, cuts=((4, 7.4), (72, 76.1), (112, 118)), offered=((163, 167.6),), markers=((96,)), playhead=4, trim=(0, 272), seed=11)}</div>
<div style="flex: 1 1 0"></div>
<div style="display: flex; flex-direction: row; align-items: center; gap: 10px">
<span style="color: {MUTED}; flex: 1 1 0">Red is cut, and playback skips it. Dashed is a suggestion. Undo puts any cut back.</span>
{button("Revert to original", colour=FG3)}{button("Done", primary=True)}</div>
</div>'''
    return page("shroom Editor, voice cuts", w, 800, body)


def mini(title, inner, w):
    return (f'<div style="width: {w}px; display: flex; flex-direction: column; gap: 10px; flex: none">'
            f'<span style="color: {MUTED}; font-size: 12px">{title}</span>{inner}</div>')


def board_doors():
    row = lambda label, colour=FG, top=True, said=None: (
        f'<button role="menuitem" style="font: inherit; height: 30px; padding: 0 10px; border: 0; border-radius: 6px; background: transparent; color: {colour}; text-align: left">{label}</button>')
    menu = f'''<div role="menu" style="width: 190px; box-sizing: border-box; background: {MENU}; border: 1px solid rgba(255, 255, 255, 0.12); border-radius: 9px; box-shadow: 0 8px 24px rgba(0, 0, 0, 0.35); padding: 4px; display: flex; flex-direction: column">
{row("Copy link")}{row("Open page")}
<button role="menuitem" style="font: inherit; height: 30px; padding: 0 10px; border: 0; border-radius: 6px; background: {BORDER}; color: {FG}; text-align: left; font-weight: 600">Edit…</button>
{row("Rename")}{row("Delete…", RED)}</div>'''
    pop = lambda inner: f'<div style="width: 300px; box-sizing: border-box; background: {BG}; border: 1px solid rgba(255, 255, 255, 0.1); border-radius: 12px; box-shadow: 0 8px 28px rgba(0, 0, 0, 0.3); padding: 14px; display: flex; flex-direction: column; gap: 12px">{inner}</div>'
    item = lambda label, icon, top=True, colour=FG, extra="": (
        f'<button style="font: inherit; height: 36px; padding: 0 10px; border: 0; {"border-top: 1px solid " + LINE + ";" if top else ""} background: {extra or "transparent"}; color: {colour}; display: flex; flex-direction: row; align-items: center; gap: 10px; text-align: left">{icon}<span>{label}</span></button>')
    sq = f'<svg viewBox="0 0 16 16" aria-hidden="true" style="width: 16px; height: 16px; fill: none; stroke: {MUTED}; stroke-width: 1.4; display: block; flex: none"><rect x="3.5" y="3.5" width="9" height="9" rx="2"></rect></svg>'
    paused = pop(f'''<h1 style="margin: 0; font-size: 15px; font-weight: 600; line-height: 28px">Paused — 4:32</h1>
<button style="font: inherit; font-weight: 600; height: 36px; border: 0; border-radius: 8px; background: {ACCENT}; color: {BG}">Resume</button>
<div style="background: {CARD}; border-radius: 9px; display: flex; flex-direction: column">
{item("Finish &amp; Share", sq, top=False)}
{item("Drop a marker", flag(ACCENT), extra="rgba(136, 192, 208, 0.12)")}
{item("Restart", sq)}{item("Discard…", sq, colour=RED)}</div>''')
    after = pop(f'''<div style="display: flex; flex-direction: row; align-items: center; gap: 8px"><span style="width: 8px; height: 8px; border-radius: 4px; background: #a3be8c; display: block"></span><span style="font-weight: 600">Link copied</span></div>
<div style="background: {CARD}; border-radius: 9px; padding: 10px 12px; display: flex; flex-direction: row; align-items: center; gap: 10px">{scissors(RED, 16)}
<span style="flex: 1 1 0">Cut 2 slips where you said “shroom cut”.</span>
<button style="font: inherit; height: 26px; padding: 0 10px; border: 1px solid {BORDER}; border-radius: 6px; background: transparent; color: {FG}">Review</button></div>
<span style="color: {MUTED}; font-size: 12px">The link already plays the cut version.</span>''')
    toggle = f'<span style="width: 34px; height: 20px; border-radius: 10px; background: {ACCENT}; position: relative; display: block; flex: none"><span style="position: absolute; right: 2px; top: 2px; width: 16px; height: 16px; border-radius: 8px; background: {FG}; display: block"></span></span>'
    settings = f'''<div style="width: 420px; box-sizing: border-box; background: {BG}; border-radius: 10px; padding: 16px; display: flex; flex-direction: column; gap: 10px">
<h1 style="margin: 0; font-size: 15px; font-weight: 600">General</h1>
<div style="background: {CARD}; border-radius: 9px; padding: 12px; display: flex; flex-direction: column; gap: 6px">
<div style="display: flex; flex-direction: row; align-items: center; gap: 10px"><span style="flex: 1 1 0; font-weight: 600">Cut slips when I say “shroom cut”</span>{toggle}</div>
<span style="color: {MUTED}; font-size: 12px">Slip, say “shroom cut”, say it again. shroom cuts the slip and the words, and you can undo it. Your voice is listened to in your own Cloudflare account, after you stop.</span>
</div></div>'''
    owner_note = (f'<div style="width: 420px; box-sizing: border-box; background: {BG}; border-radius: 10px; padding: 16px; display: flex; flex-direction: column; gap: 8px">'
                  f'<div style="display: flex; flex-direction: row; align-items: center; gap: 10px">{scissors(RED, 16)}<span style="font-weight: 600">Cut where you said “shroom cut”</span></div>'
                  f'<span style="color: {MUTED}; font-size: 12px">Under the player, for the owner and the video\'s maker only. Voice cuts run whenever the space listens (the space\'s own switch), so they add none.</span></div>')
    body = f'''<div style="flex: 1 1 0; padding: 28px; display: flex; flex-direction: row; gap: 36px; align-items: flex-start">
{mini("The Library: Edit… in a row's menu", menu, 220)}
{mini("While paused: Drop a marker comes back (hidden by B143 until the editor ships)", paused, 300)}
{mini("After Finish &amp; Share the owner page opens: voice cuts are reviewed there (“The page — cut where you said shroom cut”), with no switch of their own", owner_note, 420)}
</div>'''
    return page("shroom Editor, the ways in", 1180, 400, body)


# ── The owner page's voice cuts (N65) ─────────────────────────────────────────────────
# The page's markup is OwnerPageLive's (the canvas), with the strip under the player.

PLAY_SVG = '<svg viewBox="0 0 16 16" aria-hidden="true" style="width: 18px; height: 18px; fill: #eceff4; display: block"><path d="M4.5 2.5v11l9-5.5z"></path></svg>'


def icon_button(label, path, size=18):
    return (f'<button aria-label="{label}" style="width: 44px; height: 44px; border: 0; border-radius: 9px; background: transparent; display: flex; align-items: center; justify-content: center; flex: none">'
            f'<svg viewBox="0 0 16 16" aria-hidden="true" style="width: {size}px; height: {size}px; fill: none; stroke: #c3cad6; stroke-width: 1.4; stroke-linecap: round; stroke-linejoin: round; display: block"><path d="{path}"></path></svg></button>')


def mark_footer():
    return ('<div style="width: 100%; max-width: 960px; padding: 32px 0 0 0; display: flex; flex-direction: row; justify-content: flex-end; align-items: center; gap: 10px">'
            '<button role="switch" aria-checked="true" aria-labelledby="mark-label" style="width: 36px; height: 20px; border: 0; border-radius: 10px; background: #88c0d0; padding: 2px; display: flex; flex-direction: row; justify-content: flex-end; align-items: center; flex: none">'
            '<span style="width: 16px; height: 16px; border-radius: 8px; background: #2b2f36; display: block"></span></button>'
            '<span id="mark-label" style="color: #aeb6c4; font-size: 13px" data-said="owner-page.made-with-shroom">made with shroom</span>'
            '<svg viewBox="18 20 84 90" aria-hidden="true" style="width: 17px; height: 18px; display: block">'
            '<path d="M50,68 C49,82 46,92 46,96 C46,101 52,104 60,104 C68,104 74,101 74,96 C74,92 71,82 70,68 Z" fill="#E5E9F0"></path>'
            '<path d="M24,64 C24,42 40,26 60,26 C80,26 96,42 96,64 C86,75 34,75 24,64 Z" fill="#88C0D0"></path>'
            '<circle cx="33" cy="58" r="3.4" fill="#ECEFF4"></circle><circle cx="88" cy="58" r="2.2" fill="#ECEFF4"></circle>'
            '<circle cx="50" cy="44" r="6" fill="#ECEFF4"></circle><circle cx="72" cy="48" r="3.6" fill="#ECEFF4"></circle>'
            '<circle cx="64" cy="58" r="2.6" fill="#ECEFF4"></circle></svg></div>')


def player(now, total, played=0.3):
    return f"""<div style="width: 100%; aspect-ratio: 16 / 9; border-radius: 12px; background: #1f2228; display: flex; flex-direction: column; overflow: hidden">
<div style="flex: 1 1 0; display: flex; align-items: center; justify-content: center">
<button aria-label="Play" style="width: 72px; height: 72px; border: 0; border-radius: 36px; background: rgba(236, 239, 244, 0.16); display: flex; align-items: center; justify-content: center">
<svg viewBox="0 0 16 16" aria-hidden="true" style="width: 26px; height: 26px; fill: #eceff4; display: block"><path d="M5 3v10l8.5-5z"></path></svg>
</button>
</div>
<div style="background: #262a31; display: flex; flex-direction: column; flex: none">
<div role="slider" tabindex="0" aria-label="Seek" aria-valuetext="{now} of {total}" style="height: 16px; margin: -6px 0 -6px 0; display: flex; flex-direction: row; align-items: center; position: relative">
<div style="width: {played * 100:.0f}%; height: 4px; background: #88c0d0"></div>
<div style="width: 14px; height: 14px; border-radius: 7px; background: #eceff4; margin: 0 -7px; flex: none; position: relative"></div>
<div style="flex: 1 1 0; height: 4px; background: #434a56"></div>
</div>
<div style="height: 52px; padding: 0 10px; display: flex; flex-direction: row; align-items: center; gap: 4px; color: #d8dee9; font-size: 13px">
<button aria-label="Play" style="width: 44px; height: 44px; border: 0; border-radius: 8px; background: transparent; display: flex; align-items: center; justify-content: center; flex: none">{PLAY_SVG}</button>
<span style="font-variant-numeric: tabular-nums; padding: 0 6px; white-space: nowrap"><span style="color: #eceff4">{now}</span> / {total}</span>
<div style="flex: 1 1 0"></div>
<button aria-label="Playback speed, 1×" style="font: inherit; height: 44px; min-width: 44px; padding: 0 10px; border: 0; border-radius: 8px; background: transparent; color: #d8dee9; font-weight: 600; flex: none">1×</button>
</div>
</div>
</div>"""


def cut_row(when, said, state, note=None):
    """One cut in the strip. state: "cut" (struck, Put back), "back" (put back: plain, Cut
    again), "offer" (the line before a cue with no retake: plain, Cut it too)."""
    struck = state == "cut"
    text_style = (f"color: {MUTED}; text-decoration: line-through; text-decoration-color: {RED}" if struck else f"color: {FG}")
    label, said_id = {"cut": ("Put back", "owner-page-cuts.put-back"), "back": ("Cut again", "owner-page-cuts.cut-again"),
                      "offer": ("Cut it too", "owner-page-cuts.cut-it-too")}[state]
    note_html = f'<span style="color: {AMBER}; font-size: 13px" data-said="owner-page-cuts.only-the-words-went">{note}</span>' if note else ""
    return (f'<div style="display: flex; flex-direction: row; align-items: center; gap: 14px; padding: 10px 0 0 0; border-top: 1px solid {LINE}">'
            f'<span style="font-variant-numeric: tabular-nums; color: {FG3}; flex: none; width: 36px">{when}</span>'
            f'<span style="flex: 1 1 0; min-width: 0; display: flex; flex-direction: column; gap: 2px">{note_html}<span style="{text_style}">“{said}”</span></span>'
            f'<button style="font: inherit; font-size: 13px; height: 32px; padding: 0 12px; border: 1px solid {BORDER}; border-radius: 8px; background: transparent; color: {FG}; flex: none" data-said="{said_id}">{label}</button></div>')


OFFER_NOTE = "Nothing was said again after it, so only “shroom cut” went. Cut the line before it too?"


def strip(rows, error=None):
    err = (f'<span style="color: {AMBER}; font-size: 13px" role="status" data-said="owner-page-cuts.couldnt-save">{error}</span>' if error else "")
    return (f'<section aria-label="Cuts where you said shroom cut" style="background: {CARD}; border-radius: 12px; padding: 14px 16px; display: flex; flex-direction: column; gap: 10px">'
            f'<div style="display: flex; flex-direction: row; align-items: center; gap: 10px">{scissors(RED, 16)}'
            f'<span style="font-weight: 600; font-size: 15px" data-said="owner-page-cuts.cut-where-you-said">Cut where you said “shroom cut”</span>'
            f'<span style="flex: 1 1 0"></span><span style="color: {MUTED}; font-size: 13px" data-said="owner-page-cuts.only-you-see-this">Only you see this.</span></div>'
            f'{"".join(rows)}{err}</section>')


def owner_page(board_title, title, now, total, body_after_player, w=1280, h=1100):
    head = ('<div style="display: flex; flex-direction: row; align-items: center; gap: 4px">'
            f'<input type="text" aria-label="Title" value="{title}" style="font: inherit; font-size: 1.75rem; font-weight: 600; flex: 1 1 0; min-width: 0; box-sizing: border-box; height: 52px; padding: 0; border: 1px solid transparent; border-radius: 9px; background: transparent; color: {FG}; text-overflow: ellipsis">'
            + icon_button("Copy link", "M6.8 9.2a2.8 2.8 0 0 0 4 0l2.2-2.2a2.8 2.8 0 0 0-4-4l-.8.8M9.2 6.8a2.8 2.8 0 0 0-4 0L3 9a2.8 2.8 0 0 0 4 4l.8-.8")
            + icon_button("Download", "M8 2.5v8M4.5 7.5L8 11l3.5-3.5M3 13.5h10") + '</div>')
    body = (f'<div style="width: {w}px; min-height: {h}px; box-sizing: border-box; background: {BG}; padding: 40px 16px 24px 16px; display: flex; flex-direction: column; align-items: center; font-family: {FONT}; font-size: 14px; line-height: 1.5; color: {FG}">'
            f'<div style="width: 100%; max-width: 960px; flex: 1 1 auto; display: flex; flex-direction: column; gap: 20px">{head}{player(now, total)}{body_after_player}</div>'
            f'{mark_footer()}</div>')
    return f"""<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<title>{board_title}</title>
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
{body}
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


def board_owner_cuts():
    rows = [
        cut_row("0:04", "Hello, that is shroom. Shroom cut.", "cut"),
        cut_row("1:12", "Then you pick the, the window you, um, shroom cut.", "cut"),
        cut_row("2:47", "The second one is, um, I forget what it's called.", "offer", OFFER_NOTE),
    ]
    return owner_page("shroom video page, cut where you said shroom cut", "Setting up shroom in two minutes", "1:21", "3:58", strip(rows))


def board_owner_cut_rows():
    """The strip's rows after a press, and when a press didn't save (the row goes back)."""
    def panel(caption, inner):
        return (f'<div style="display: flex; flex-direction: column; gap: 10px; width: 600px">'
                f'<span style="color: {MUTED}; font-size: 12px">{caption}</span>{inner}</div>')
    a = panel("Put back: the slip plays again, and the row offers Cut again",
              strip([cut_row("0:04", "Hello, that is shroom. Shroom cut.", "back")]))
    b = panel("Cut it too: the line before the cue is cut as well, and can be put back",
              strip([cut_row("2:47", "The second one is, um, I forget what it's called.", "cut")]))
    c = panel("A press that didn't save: the row is as it was, and one line says so",
              strip([cut_row("1:12", "Then you pick the, the window you, um, shroom cut.", "cut")],
                    error="Couldn't save that. Check your connection, then try again."))
    body = (f'<div style="width: 1320px; min-height: 560px; box-sizing: border-box; background: #1f2228; padding: 28px; display: grid; grid-template-columns: 600px 600px; gap: 36px 60px; font-family: {FONT}; font-size: 14px; line-height: 1.5; color: {FG}">'
            f'{a}{b}{c}</div>')
    return page("shroom video page, the cut rows after a press", 1320, 560, body)


def main():
    root = Path(sys.argv[1]) / "project"
    root.mkdir(parents=True, exist_ok=True)
    (root / "EditorTrim.dc.html").write_text(board_trim())
    (root / "EditorVoiceCuts.dc.html").write_text(board_cues())
    (root / "EditorDoors.dc.html").write_text(board_doors())
    (root / "OwnerPageCuts.dc.html").write_text(board_owner_cuts())
    (root / "OwnerPageCutRows.dc.html").write_text(board_owner_cut_rows())


if __name__ == "__main__":
    main()
