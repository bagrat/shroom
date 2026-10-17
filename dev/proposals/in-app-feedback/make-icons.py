#!/usr/bin/env python3
"""Writes the icon sheet for A's feedback entry: eight 16-px glyphs on the popover icons' grid
(stroke 1.4, round caps), each in the header beside Library · Settings · Quit, dark and light,
at real size and at 3×. Then `node render.mjs` for the PNG."""
from pathlib import Path

OUT = Path(__file__).parent
J = 'stroke-linejoin="round"'
BUBBLE = f'<path {J} d="M3 3h10a1.5 1.5 0 0 1 1.5 1.5v5.5a1.5 1.5 0 0 1-1.5 1.5H7.5L4.5 14v-2.5H3A1.5 1.5 0 0 1 1.5 10V4.5A1.5 1.5 0 0 1 3 3z"></path>'
OPTIONS = [
    ("1", "Speech bubble", BUBBLE),
    ("2", "Bubble with lines", BUBBLE + '<path d="M5 6.3h6M5 8.7h3.5"></path>'),
    ("3", "Bubble with a dot", BUBBLE + '<circle cx="5.2" cy="7.25" r=".5" fill="currentColor"></circle><circle cx="8" cy="7.25" r=".5" fill="currentColor"></circle><circle cx="10.8" cy="7.25" r=".5" fill="currentColor"></circle>'),
    ("4", "Heart in a bubble", BUBBLE + f'<path {J} d="M8 9.4L5.9 7.4a1.2 1.2 0 0 1 2.1-1.5a1.2 1.2 0 0 1 2.1 1.5z"></path>'),
    ("5", "Envelope", f'<rect {J} x="1.5" y="3.5" width="13" height="9" rx="1.6"></rect><path {J} d="M2.2 4.4L8 8.6l5.8-4.2"></path>'),
    ("6", "Paper plane", f'<path {J} d="M14.5 1.8L1.5 7l4.6 1.9L14.5 1.8zM6.1 8.9l1.8 5.3 6.6-12.4"></path>'),
    ("7", "Megaphone", f'<path {J} d="M2 6.3v3.4h2.6L11 13V3L4.6 6.3z"></path><path d="M4.6 9.7l.9 3.6M13.2 6.3v3.4"></path>'),
    ("8", "Smiley", '<circle cx="8" cy="8" r="6.3"></circle><path d="M5.5 9.6a3 3 0 0 0 5 0"></path><path d="M6 6.2v.5M10 6.2v.5"></path>'),
]
OTHERS = {
    "Library": '<path d="M4.5 2.5h7"></path><rect x="2" y="5" width="12" height="9" rx="1.8"></rect><path d="M7 7.8v3.4l2.8-1.7z" stroke-linejoin="round"></path>',
    "Settings": '<circle cx="8" cy="8" r="2"></circle><path stroke-linejoin="round" d="M12.54 6.16L14.46 6.63L14.46 9.37L12.54 9.84L11.86 11.02L12.42 12.9L10.04 14.28L8.68 12.85L7.32 12.85L5.96 14.28L3.58 12.9L4.14 11.02L3.46 9.84L1.54 9.37L1.54 6.63L3.46 6.16L4.14 4.98L3.58 3.1L5.96 1.72L7.32 3.15L8.68 3.15L10.04 1.72L12.42 3.1L11.86 4.98Z"></path>',
    "Quit shroom": '<path d="M8 2v6M4.5 4.5a5 5 0 1 0 7 0"></path>',
}
THEME = {
    "dark": dict(bg="#2b2f36", icon="#aeb6c4", fg="#eceff4", edge="rgba(255, 255, 255, 0.1)", grid="#3b4252"),
    "light": dict(bg="#f6f7f9", icon="#4c566a", fg="#2e3440", edge="#dfe3ea", grid="#e3e7ee"),
}
MUSH = ('<svg viewBox="18 20 84 90" aria-hidden="true" style="width: {s}px; height: {t}px; display: block">'
        '<path d="M50,68 C49,82 46,92 46,96 C46,101 52,104 60,104 C68,104 74,101 74,96 C74,92 71,82 70,68 Z" fill="#E5E9F0"></path>'
        '<path d="M24,64 C24,42 40,26 60,26 C80,26 96,42 96,64 C86,75 34,75 24,64 Z" fill="#88C0D0"></path>'
        '<circle cx="50" cy="44" r="6" fill="#ECEFF4"></circle><circle cx="72" cy="48" r="3.6" fill="#ECEFF4"></circle></svg>')


def glyph(d, color, px):
    return (f'<svg viewBox="0 0 16 16" aria-hidden="true" style="width: {px}px; height: {px}px; fill: none; stroke: {color}; '
            f'stroke-width: 1.4; stroke-linecap: round; display: block">{d.replace("currentColor", color)}</svg>')


def header(d, t, k):
    """The popover's header at scale k (1 or 3)."""
    btn = lambda label, g: (f'<button aria-label="{label}" style="width: {28*k}px; height: {28*k}px; border: 0; background: transparent; '
                            f'padding: 0; display: flex; align-items: center; justify-content: center">{glyph(g, t["icon"], 16*k)}</button>')
    icons = btn("Send feedback", d) + "".join(btn(n, g) for n, g in OTHERS.items())
    return (f'<div style="width: {272*k}px; box-sizing: border-box; background: {t["bg"]}; border: {k}px solid {t["edge"]}; border-radius: {10*k}px; '
            f'padding: {8*k}px {10*k}px; display: flex; flex-direction: row; justify-content: space-between; align-items: center; color: {t["fg"]}">'
            f'<div style="display: flex; flex-direction: row; align-items: center; gap: {7*k}px">{MUSH.format(s=17*k, t=18*k)}'
            f'<div style="font-size: {14*k}px; font-weight: 600">shroom</div></div>'
            f'<div style="display: flex; flex-direction: row; gap: {2*k}px">{icons}</div></div>')


def cell(num, name, d):
    big = header(d, THEME["dark"], 3)
    small = "".join(f'<div style="display: flex; flex-direction: column; gap: 4px"><div style="color: #7b8494; font-size: 11px">{m}</div>{header(d, THEME[m], 1)}</div>'
                    for m in ("dark", "light"))
    tiles = "".join(f'<div style="width: 64px; height: 64px; border-radius: 10px; background: {THEME[m]["bg"]}; border: 1px solid {THEME[m]["edge"]}; '
                    f'display: flex; align-items: center; justify-content: center; background-image: linear-gradient({THEME[m]["grid"]} 1px, transparent 1px), '
                    f'linear-gradient(90deg, {THEME[m]["grid"]} 1px, transparent 1px); background-size: 3px 3px; background-position: 8px 8px">'
                    f'{glyph(d, THEME[m]["icon"], 48)}</div>' for m in ("dark", "light"))
    return (f'<div style="width: 860px; box-sizing: border-box; background: #22262c; border-radius: 12px; padding: 18px; display: flex; flex-direction: column; gap: 14px">'
            f'<div style="font-size: 15px; font-weight: 600"><span style="color: #88c0d0">{num}</span> · {name}</div>'
            f'<div style="display: flex; flex-direction: row; gap: 18px; align-items: flex-start">{small}<div style="display: flex; flex-direction: row; gap: 10px">{tiles}</div></div>'
            f'{big}</div>')


DOC = """<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<title>shroom Send feedback, icon options</title>
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
<div style="width: {W}px; height: {H}px; box-sizing: border-box; background: #1a1d22; padding: 30px; display: flex; flex-direction: column; gap: 22px; font-family: -apple-system, BlinkMacSystemFont, system-ui, sans-serif; font-size: 13px; line-height: 1.5; color: #eceff4">
<div style="display: flex; flex-direction: column; gap: 4px"><div style="font-size: 20px; font-weight: 600">{title}</div><div style="color: #aeb6c4; max-width: 1500px">{intro}</div></div>
<div style="display: grid; grid-template-columns: {grid}; gap: 40px 60px">{rows}</div>
</div>
</x-dc>
<script type="text/x-dc" data-dc-script data-props='{{"$preview":{{"width":{W},"height":{H}}}}}'>
class Component extends DCLogic {{
renderVals() {{
return {{}};
}}
}}
</script>
</body>
</html>
"""


def sheet(name, title, intro, options, cols, W, H):
    rows = "".join(cell(*o) for o in options)
    grid = " ".join(["860px"] * cols)
    doc = DOC.format(W=W, H=H, title=title, intro=intro, rows=rows, grid=grid)
    (OUT / f"{name}.dc.html").write_text(doc)


MEGA = f'<path {J} d="M2 6.3v3.4h2.6L11 13V3L4.6 6.3z"></path><path d="M4.6 9.7l.9 3.6M13.2 6.3v3.4"></path>'
SMALL_MEGA = f'<path {J} d="M1.5 6.5v3h2.3L9 12.3V3.7L3.8 6.5z"></path><path d="M3.8 9.5l.8 3.2"></path>'
MORE = [
    ("9", "Megaphone with sound waves", SMALL_MEGA + '<path d="M11.2 6a2.6 2.6 0 0 1 0 4M13 4.4a5 5 0 0 1 0 7.2"></path>'),
    ("10", "Megaphone, tilted up", f'<g transform="rotate(-20 8 8)">{MEGA}</g>'),
    ("11", "Bullhorn with a handle", f'<path {J} d="M2.5 6v4l7.5 3.5V2.5z"></path><path d="M10 2.5v11"></path><path {J} d="M5 11.2v2.3h2.2"></path>'),
    ("12", "Megaphone, minimal", f'<path {J} d="M2 6.5h2.5L12 2.5v11L4.5 9.5H2z"></path>'),
    ("13", "Megaphone, filled", f'<g fill="currentColor">{MEGA}</g>'),
    ("14", "Loudspeaker with waves", f'<path {J} d="M2 6h2.5L8 3v10L4.5 10H2z"></path><path d="M10.5 6a2.8 2.8 0 0 1 0 4M12.5 4a5.6 5.6 0 0 1 0 8"></path>'),
    ("15", "Megaphone in a bubble", BUBBLE + f'<g transform="translate(4 3.6) scale(.5)" style="stroke-width: 2.6">{MEGA}</g>'),
    ("16", "Megaphone and a small bubble", f'<g transform="translate(0 2) scale(.82)">{MEGA}</g><path {J} d="M11 1.6h3a1 1 0 0 1 1 1v2a1 1 0 0 1-1 1h-1.4L11.4 6.6V5.6H11a1 1 0 0 1-1-1v-2a1 1 0 0 1 1-1z"></path>'),
    ("17", "Broadcast", '<circle cx="8" cy="8" r="1.3"></circle><path d="M5.2 5.2a4 4 0 0 0 0 5.6M10.8 5.2a4 4 0 0 1 0 5.6M3.1 3.1a7 7 0 0 0 0 9.8M12.9 3.1a7 7 0 0 1 0 9.8"></path>'),
    ("18", "Megaphone, facing left", f'<g transform="matrix(-1 0 0 1 16 0)">{MEGA}</g>'),
    ("19", "Megaphone with shout lines", SMALL_MEGA + '<path d="M11.5 5l2-1.4M12 8h2.3M11.5 11l2 1.4"></path>'),
    ("20", "Megaphone, rounded", f'<path {J} d="M2 6.5a1 1 0 0 1 1-1h1.8l6-2.8a.8.8 0 0 1 1.2.7v9.2a.8.8 0 0 1-1.2.7l-6-2.8H3a1 1 0 0 1-1-1z"></path><path d="M5 10.4l.8 2.8"></path>'),
    ("21", "Question mark in a circle", '<circle cx="8" cy="8" r="6.3"></circle><path d="M6.2 6.3a1.9 1.9 0 0 1 3.7.6c0 1.3-1.9 1.6-1.9 2.8"></path><circle cx="8" cy="11.4" r=".5" fill="currentColor"></circle>'),
    ("22", "Question mark in a bubble", BUBBLE + '<path d="M6.5 5.6a1.6 1.6 0 0 1 3.1.5c0 1-1.6 1.2-1.6 2.1"></path><circle cx="8" cy="9.6" r=".45" fill="currentColor"></circle>'),
    ("23", "A hand, waving", f'<path {J} d="M5 9V4.5a1 1 0 0 1 2 0V8M7 7.5V3.5a1 1 0 0 1 2 0v4M9 7.5V4.5a1 1 0 0 1 2 0V9.5c0 2.5-1.8 4.5-4.2 4.5-1.6 0-2.6-.8-3.4-2L2.6 9.8a1 1 0 0 1 1.6-1.2L5 9.5"></path><path d="M12.8 2.3a3 3 0 0 1 1.3 2"></path>'),
    ("24", "Pencil in a bubble", BUBBLE + f'<path {J} d="M5.8 9.3l.3-1.5 3.2-3.2a.75.75 0 0 1 1.1 1.1L7.2 8.9z"></path>'),
    ("25", "Sparkle", f'<path {J} d="M8 2l1.4 4.6L14 8l-4.6 1.4L8 14l-1.4-4.6L2 8l4.6-1.4z"></path>'),
    ("26", "Mushroom in a bubble", BUBBLE + f'<path {J} d="M4.8 7.6a3.2 3.2 0 0 1 6.4 0z"></path><path {J} d="M7.1 7.6v1.8h1.8V7.6"></path>'),
    ("27", "Lightbulb", '<path d="M6 11.5h4M6.5 13.6h3M6 11.5c0-1.6-2-2.6-2-5a4 4 0 0 1 8 0c0 2.4-2 3.4-2 5"></path>'),
    ("28", "Heart", f'<path {J} d="M8 13.5L2.9 8.6a3 3 0 0 1 4.3-4.2L8 5.2l.8-.8a3 3 0 0 1 4.3 4.2z"></path>'),
]

sheet("FeedbackIcons", "A · Send feedback — the icon, eight options",
      "Each sits first in the popover's header, before Library · Settings · Quit: at real size in dark and light, as a 3× glyph on the 16-px grid, and the dark header at 3×. Every glyph is drawn as its neighbours are: 16 × 16, a 1.4 stroke, round caps and joins, no fill. Its name on hover and for VoiceOver stays “Send feedback”.",
      OPTIONS, 2, 1840, 1430)
sheet("FeedbackIcons2", "A · Send feedback — the icon, twenty more (9–28)",
      "Bagrat called the megaphone (7) the most fitting: 9–20 are its family — sound waves, angles, a bullhorn, minimal and filled, a loudspeaker, a broadcast, and hybrids with a bubble. 21–22 are the question mark he expects reads as help, for comparison; 23–28 are fresh ideas, a mushroom in a bubble among them. Same format as 1–8: in the header in dark and light at real size, as a 3× glyph on the 16-px grid, and the dark header at 3×.",
      MORE, 3, 2740, 2440)
