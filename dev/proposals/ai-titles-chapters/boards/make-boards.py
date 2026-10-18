#!/usr/bin/env python3
"""The naming boards for the shroom UI canvas (N66, N67; DECISIONS B205–B209), as built.

Each board is an x-dc page in the canvas's own style (Nord tokens, as RecipientLive,
OwnerPage, SettingsSpace and LibraryRows are drawn), showing what the code draws: the
Library's card (app/src/Library.svelte), the switch in Settings · Space
(app/src/settings/Space.svelte), and the page's title line, chapters and captions
(worker/src/worker.js; tests/e2e/pages-gallery.mjs shoots the same faces). Each line the
spec says carries its id. Usage: make-boards.py <canvas project dir> writes the boards there
and places them in canvas.json, in their own column."""
import json, sys, os

out = sys.argv[1]
FONT = "-apple-system, BlinkMacSystemFont, system-ui, sans-serif"
MARK = '''<svg viewBox="18 20 84 90" aria-hidden="true" style="width: 17px; height: 18px; display: block">
<path d="M50,68 C49,82 46,92 46,96 C46,101 52,104 60,104 C68,104 74,101 74,96 C74,92 71,82 70,68 Z" fill="#E5E9F0"></path>
<path d="M24,64 C24,42 40,26 60,26 C80,26 96,42 96,64 C86,75 34,75 24,64 Z" fill="#88C0D0"></path>
<circle cx="50" cy="44" r="6" fill="#ECEFF4"></circle><circle cx="72" cy="48" r="3.6" fill="#ECEFF4"></circle><circle cx="33" cy="58" r="3.4" fill="#ECEFF4"></circle><circle cx="64" cy="58" r="2.6" fill="#ECEFF4"></circle><circle cx="88" cy="58" r="2.2" fill="#ECEFF4"></circle>
</svg>'''
SPARK = '<path d="M8 2.5l1.3 3.2 3.2 1.3-3.2 1.3L8 11.5 6.7 8.3 3.5 7l3.2-1.3z"></path>'
SW_ON = '<button role="switch" aria-checked="true" aria-labelledby="{id}" style="width: 36px; height: 20px; border: 0; border-radius: 10px; background: #88c0d0; padding: 2px; display: flex; flex-direction: row; justify-content: flex-end; align-items: center; flex: none"><span style="width: 16px; height: 16px; border-radius: 8px; background: #2b2f36; display: block"></span></button>'
SW_OFF = '<button role="switch" aria-checked="false" aria-labelledby="{id}" style="width: 36px; height: 20px; border: 0; border-radius: 10px; background: #4c566a; padding: 2px; display: flex; flex-direction: row; justify-content: flex-start; align-items: center; flex: none"><span style="width: 16px; height: 16px; border-radius: 8px; background: #d8dee9; display: block"></span></button>'

def page(title, w, h, body):
    return f'''<!doctype html>
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
'''

def note(text):
    return f'<div style="padding: 10px 12px; border: 1px dashed #4c566a; border-radius: 9px; color: #aeb6c4; font-size: 12px; line-height: 1.5">{text}</div>'

# ───────── the page ─────────

# The pages gallery's named take (invented script; the chapters as the naming step placed them).
TITLE = "Sign up screen, invoice bug, trial numbers"
DUR = 183
CHAPTERS = [(0, "Sign up screen changes"), (68, "Invoice page bug details"), (114, "Trial results and metrics"), (157, "Next steps and wrap up")]
mmss = lambda t: f"{t // 60}:{t % 60:02d}"
chapter_at = lambda t: [c for s, c in CHAPTERS if s <= t][-1]

def icon(d, stroke="#d8dee9", fill="none", size=18):
    return f'<svg viewBox="0 0 16 16" aria-hidden="true" style="width: {size}px; height: {size}px; fill: {fill}; stroke: {stroke}; stroke-width: 1.4; stroke-linecap: round; stroke-linejoin: round; display: block"><path d="{d}"></path></svg>'

def row_button(label, svg):
    return f'<button aria-label="{label}" style="width: 44px; height: 44px; border: 0; border-radius: 8px; background: transparent; display: flex; align-items: center; justify-content: center; flex: none">{svg}</button>'

PLAY = '<svg viewBox="0 0 16 16" aria-hidden="true" style="width: 18px; height: 18px; fill: #eceff4; display: block"><path d="M4.5 2.5v11l9-5.5z"></path></svg>'
PAUSE = '<svg viewBox="0 0 16 16" aria-hidden="true" style="width: 18px; height: 18px; fill: #eceff4; display: block"><path d="M4 3h3v10H4zM9 3h3v10H9z"></path></svg>'
SOUND = '<svg viewBox="0 0 16 16" aria-hidden="true" style="width: 18px; height: 18px; fill: none; stroke: #d8dee9; stroke-width: 1.4; stroke-linecap: round; stroke-linejoin: round; display: block"><path d="M2 6.2h2.3L7.5 3.5v9L4.3 9.8H2z"></path><path d="M10 6a3 3 0 0 1 0 4M11.8 4a5.6 5.6 0 0 1 0 8"></path></svg>'
FULL = icon("M2.5 6V3.5H5M11 3.5h2.5V6M13.5 10v2.5H11M5 12.5H2.5V10")
def CC(on):
    c = "#88c0d0" if on else "#d8dee9"
    return f'<svg viewBox="0 0 16 16" aria-hidden="true" style="width: 18px; height: 18px; fill: none; stroke: {c}; stroke-width: 1.4; stroke-linecap: round; stroke-linejoin: round; display: block"><rect x="1.5" y="3" width="13" height="10" rx="2"></rect><path d="M7.2 6.6a1.8 1.8 0 1 0 0 2.8M11.7 6.6a1.8 1.8 0 1 0 0 2.8"></path></svg>'
LINK = icon("M6.8 9.2a2.8 2.8 0 0 0 4 0l2.2-2.2a2.8 2.8 0 0 0-4-4l-.8.8M9.2 6.8a2.8 2.8 0 0 0-4 0L3 9a2.8 2.8 0 0 0 4 4l.8-.8", stroke="#c3cad6")
THUMB = icon("M8.2 12.8H3.3a1.5 1.5 0 0 1-1.5-1.5V4.7a1.5 1.5 0 0 1 1.5-1.5h9.4a1.5 1.5 0 0 1 1.5 1.5v3.5M7 6.2v3.6l3-1.8zM11.89 12.95a1.232 1.232 0 0 0 1.76 0l.968-.968a1.232 1.232 0 0 0-1.76-1.76l-.352.352M12.95 11.89a1.232 1.232 0 0 0-1.76 0L10.22 12.86a1.232 1.232 0 0 0 1.76 1.76l.352-.352", stroke="#c3cad6")

def head_button(label, svg):
    return f'<button aria-label="{label}" style="width: 44px; height: 44px; border: 0; border-radius: 9px; background: transparent; display: flex; align-items: center; justify-content: center; flex: none">{svg}</button>'

def seekbar(now, chapters, tip=None, held=False):
    """The seek bar along the controls' top edge. With chapters: one section each, 2 px apart,
    played part lit; `tip`: (seconds, the pointer's or finger's place) names that chapter above
    the bar, its section grown. `held`: a finger on the thumb, which grows."""
    knob = now / DUR * 100
    if chapters:
        segs = []
        grown = None if tip is None else [i for i, (s, _) in enumerate(CHAPTERS) if s <= tip][-1]
        for i, (s, t) in enumerate(CHAPTERS):
            e = CHAPTERS[i + 1][0] if i + 1 < len(CHAPTERS) else DUR
            played = max(0, min(e, now) - s) / (e - s) * 100
            segs.append(f'<div style="flex: {e - s} 1 0; height: {7 if grown == i else 4}px; border-radius: 2px; background: linear-gradient(to right, #88c0d0 {played:.1f}%, #434a56 {played:.1f}%)"></div>')
        track = f'<div style="position: absolute; left: 0; right: 0; top: 6px; height: 4px; display: flex; flex-direction: row; align-items: center; gap: 2px">{"".join(segs)}</div>'
    else:
        track = f'<div style="position: absolute; left: 0; right: 0; top: 6px; height: 4px; background: linear-gradient(to right, #88c0d0 {knob:.1f}%, #434a56 {knob:.1f}%)"></div>'
    size = 20 if held else 14
    thumb = f'<div style="position: absolute; left: calc({knob:.2f}% - {size // 2}px); top: {8 - size // 2}px; width: {size}px; height: {size}px; border-radius: {size // 2}px; background: #eceff4{"; box-shadow: 0 0 0 6px rgba(236, 239, 244, 0.18)" if held else ""}"></div>'
    label = f"{mmss(now)} / {mmss(DUR)}" + (f", {chapter_at(now)}" if chapters else "")
    tipped = ""
    if tip is not None:
        at = tip / DUR * 100
        tipped = f'<div style="position: absolute; left: {at:.1f}%; bottom: 24px; transform: translateX(-50%); background: #1f2228; border: 1px solid #434a56; border-radius: 7px; padding: 4px 9px; white-space: nowrap; font-size: 12px; line-height: 1.35; color: #eceff4"><span style="color: #d8dee9; font-variant-numeric: tabular-nums; margin-right: 6px">{mmss(tip)}</span>{chapter_at(tip)}</div>'
    return f'''<div role="slider" tabindex="0" aria-label="Seek" aria-valuemin="0" aria-valuemax="{DUR}" aria-valuenow="{now}" aria-valuetext="{label}" style="height: 16px; margin: -6px 0 -6px 0; position: relative">
{track}
{thumb}
{tipped}
</div>'''

def player(now, chapters=True, tip=None, held=False, playing=False, started=True, cc=None, cue=None, phone=False):
    """The player as the page draws it: the picture (a big play until the first play), then
    the controls: the seek bar, play, the time, the chapter playing (where the row has room),
    Captions when the video has them, speed, sound, full screen. `cc`: None (no captions),
    False (off), True (on)."""
    big = '' if started else '''<button aria-label="Play" style="width: 72px; height: 72px; border: 0; border-radius: 36px; background: rgba(236, 239, 244, 0.16); display: flex; align-items: center; justify-content: center">
<svg viewBox="0 0 16 16" aria-hidden="true" style="width: 26px; height: 26px; fill: #eceff4; display: block"><path d="M5 3v10l8.5-5z"></path></svg>
</button>'''
    shown = f'<div style="position: absolute; left: 50%; bottom: 12px; transform: translateX(-50%); background: rgba(0, 0, 0, 0.8); color: #ffffff; font-size: 22px; line-height: 1.3; padding: 1px 6px; white-space: nowrap">{cue}</div>' if cue else ''
    name = f'<span style="color: #d8dee9; opacity: 0.8; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; min-width: 0; flex: 0 1 auto">· {chapter_at(now)}</span>' if chapters and not phone else ''
    cc_button = '' if cc is None else row_button("Captions", CC(cc))
    return f'''<div style="width: 100%; aspect-ratio: 16 / 9; border-radius: {10 if phone else 12}px; background: #1f2228; display: flex; flex-direction: column; overflow: hidden">
<div style="flex: 1 1 0; display: flex; align-items: center; justify-content: center; position: relative">{big}{shown}</div>
<div style="background: #262a31; display: flex; flex-direction: column; flex: none; position: relative">
{seekbar(now, chapters, tip, held)}
<div style="height: 52px; padding: 0 10px; display: flex; flex-direction: row; align-items: center; gap: 4px; color: #d8dee9; font-size: 13px">
{row_button("Pause" if playing else "Play", PAUSE if playing else PLAY)}
<span style="font-variant-numeric: tabular-nums; padding: 0 6px; white-space: nowrap; flex: none"><span style="color: #eceff4">{mmss(now)}</span> / {mmss(DUR)}</span>
{name}
<div style="flex: 1 1 0"></div>
{cc_button}
<button aria-label="Playback speed, 1×" style="font: inherit; height: 44px; min-width: 44px; padding: 0 10px; border: 0; border-radius: 8px; background: transparent; color: #d8dee9; font-weight: 600; flex: none">1×</button>
{row_button("Mute", SOUND)}
{row_button("Full screen", FULL)}
</div>
</div>
</div>'''

def owner_head(title, line, line_id):
    """The owner's title field: while nobody has typed a title it is a field holding the
    space's name (or the date) as its placeholder; the line under it says where it came from."""
    field = f'<input type="text" aria-label="Title" placeholder="{title}" style="font: inherit; font-size: 1.75rem; font-weight: 600; flex: 1 1 0; min-width: 0; box-sizing: border-box; height: 52px; padding: 0 12px; border: 1px solid #4c566a; border-radius: 9px; background: #262a31; color: #eceff4; text-overflow: ellipsis">'
    return f'''<div style="display: flex; flex-direction: row; align-items: center; gap: 4px">
{field}
{head_button("Copy link", LINK)}
{head_button("Copy thumbnail", THUMB)}
</div>
<p style="margin: -12px 0 0 0; color: #c3cad6; font-size: 15px" data-said="{line_id}">{line}</p>'''

def viewer_head(title):
    return f'''<div style="display: flex; flex-direction: row; align-items: center; gap: 4px; min-height: 52px">
<h1 style="margin: 0; font-size: 1.75rem; font-weight: 600; line-height: 1.25; flex: 1 1 0; min-width: 0">{title}</h1>
{head_button("Copy link", LINK)}
</div>'''

OWNER_FOOT = f'''<div style="width: 100%; max-width: 960px; padding: 32px 0 0 0; display: flex; flex-direction: row; justify-content: flex-end; align-items: center; gap: 10px">
{SW_ON.format(id="mark-label")}
<span id="mark-label" style="color: #aeb6c4; font-size: 13px" data-said="owner-page.made-with-shroom">made with shroom</span>
{MARK}
</div>'''
VIEWER_FOOT = f'''<div style="width: 100%; max-width: 960px; padding: 32px 0 0 0; display: flex; flex-direction: row; justify-content: flex-end">
<a href="https://shroom.am" style="display: flex; flex-direction: row; align-items: center; gap: 8px; font-size: 13px; text-decoration: none; min-height: 44px">{MARK}<span data-said="live.made-with-shroom">made with shroom</span></a>
</div>'''

def web(inner, foot, extra=""):
    return f'''<div style="min-height: 100vh; box-sizing: border-box; background: #2b2f36; padding: 40px 16px 24px 16px; display: flex; flex-direction: column; align-items: center; font-family: {FONT}; font-size: 14px; line-height: 1.5; color: #eceff4">
<div style="width: 100%; max-width: 960px; flex: 1 1 auto; display: flex; flex-direction: column; gap: 20px">
{inner}
{extra}
</div>
{foot}
</div>'''

boards = {}

def timeline():
    """When the name comes (Bagrat, 2026-10-18): a title from the first minute while recording,
    so the page opens named; the full name and chapters at Stop replace it, quietly."""
    steps = [("0:00", "Recording starts", "The page would show the date as its title."),
             ("a minute in", "The first minute is heard", "A title from what was said so far is kept for the page, seconds later."),
             ("Stop", "The page opens", "With that title, in the field's placeholder."),
             ("10–20 s later", "The full name and chapters", "Replace it in place, quietly: no flash. A viewer sees it on their next visit.")]
    cells = "".join(f'''<div style="flex: 1 1 0; display: flex; flex-direction: column; gap: 4px; position: relative">
<div style="display: flex; flex-direction: row; align-items: center; gap: 8px"><span style="width: 10px; height: 10px; border-radius: 5px; background: {'#88c0d0' if i in (1, 3) else '#4c566a'}; display: block; flex: none"></span><span style="font-variant-numeric: tabular-nums; color: #88c0d0; font-size: 12px; font-weight: 600">{t}</span></div>
<div style="color: #eceff4; font-size: 13px; font-weight: 600">{a}</div>
<div style="color: #aeb6c4; font-size: 12px; line-height: 1.45">{b}</div>
</div>''' for i, (t, a, b) in enumerate(steps))
    return f'''<div style="padding: 12px 14px; border: 1px dashed #4c566a; border-radius: 9px; display: flex; flex-direction: column; gap: 10px">
<div style="color: #aeb6c4; font-size: 12px">When the name comes</div>
<div style="display: flex; flex-direction: row; gap: 18px">{cells}</div>
<div style="color: #aeb6c4; font-size: 12px; line-height: 1.5">A title someone types, at any point, wins for good. A take shorter than a minute, or one with no speech in its first minute, is named once, at Stop. A slip cut with “shroom cut” in the first minute never reaches the early title.</div>
</div>'''

boards["AiOwnerNamed.dc.html"] = (
    "The page — the owner's, named from what was said: a title from the first minute, the full name at Stop (N67; words are drafts)", 1280, 980,
    page("shroom video page, owner, named", 1280, 980, web(
        owner_head(TITLE, "Named from what you said. Click the title to change it.", "owner-page.named-from-what-you-said") + "\n" + player(0, started=False, cc=False),
        OWNER_FOOT,
        timeline())))

LATER_DAY = "Untitled — Oct 18"
boards["AiNamingLater.dc.html"] = (
    "The page — the owner's, its name waiting for the free hours (N66, N67; words are drafts)", 1280, 900,
    page("shroom video page, owner, naming later", 1280, 900, web(
        owner_head(LATER_DAY, "Its name and chapters come a little later, once your free hours are back.", "owner-page.its-name-and-chapters-come") + "\n" + player(0, chapters=False, started=False),
        OWNER_FOOT,
        note("The account's free AI for the day is used up: the take waits for the next day (00:00 UTC) and is named then. Nothing reads as an error; until then the title is the date, the bar is whole and there are no captions."))))

boards["AiPageMarks.dc.html"] = (
    "The page — chapters in the seek bar, named on hover (Bagrat's pick, 2026-10-18)", 1280, 900,
    page("shroom video page, chapters in the seek bar", 1280, 900, web(
        viewer_head(TITLE) + "\n" + player(80, tip=122, playing=True, cc=False),
        VIEWER_FOOT,
        note("The bar is cut into one section per chapter, 2 px apart. Hovering it, playing or paused, shows the time and the chapter's name at the pointer, and that section grows a little. The chapter playing is named after the time, and the bar's spoken value names it (“1:20 / 3:03, Invoice page bug details”). Only for a video with 2 or more chapters (90 s or longer); otherwise the bar is whole, as before. Phones: the next board."))))

def phone_body(now):
    return f'''<div style="width: 390px; height: 844px; box-sizing: border-box; background: #2b2f36; padding: 40px 16px 24px 16px; display: flex; flex-direction: column; font-family: {FONT}; font-size: 14px; line-height: 1.5; color: #eceff4">
<div style="flex: 1 1 auto; display: flex; flex-direction: column; gap: 20px">
{viewer_head(TITLE)}
{player(now, tip=now, held=True, cc=False, phone=True)}
{note("No hover on a phone: a finger on the bar shows the time and chapter above the thumb, and the thumb grows while the finger is on it; the tip stays about 1.5 s after a tap. The bar's touch area is 44 px tall. The row has no room for the chapter's name beside its five controls, so the tip and the bar's spoken value name it. (Changed from the board as approved, where the name sat after the time: found while building.)")}
</div>
<div style="display: flex; flex-direction: row; justify-content: flex-end; padding: 24px 0 0 0">
<a href="https://shroom.am" style="display: flex; flex-direction: row; align-items: center; gap: 8px; font-size: 13px; text-decoration: none; min-height: 44px">{MARK}<span data-said="live.made-with-shroom">made with shroom</span></a>
</div>
</div>'''
boards["AiPageMarksPhone.dc.html"] = (
    "The page on a phone — chapters in the seek bar, named while a finger is on it (approved 2026-10-18; the name after the time dropped, no room)", 390, 844,
    page("shroom video page on a phone, chapters", 390, 844, phone_body(122)))

boards["AiCaptions.dc.html"] = (
    "The page — captions on (N67)", 1280, 900,
    page("shroom video page, captions on", 1280, 900, web(
        viewer_head(TITLE) + "\n" + player(70, playing=True, cc=True, cue="Next, the invoice page bug."),
        VIEWER_FOOT,
        note("Captions, when the space has them: the button before the speed turns them on, off until pressed and remembered on that browser. They're written from the same words the name came from, follow the saved edit, and a line never crosses a cut."))))

# Option A, not chosen: kept for the record, as it was drawn.
def chapter_list(now):
    rows = []
    for i, (s, t) in enumerate(CHAPTERS):
        e = CHAPTERS[i + 1][0] if i + 1 < len(CHAPTERS) else DUR
        on = s <= now < e
        rows.append(f'''<button aria-current="{'true' if on else 'false'}" style="font: inherit; min-height: 44px; padding: 0 12px; border: 0; {'border-top: 1px solid #434a56; ' if i else ''}background: {'rgba(136, 192, 208, 0.12)' if on else 'transparent'}; color: #eceff4; display: flex; flex-direction: row; align-items: center; gap: 14px; text-align: left">
<span style="font-variant-numeric: tabular-nums; color: {'#88c0d0' if on else '#aeb6c4'}; width: 36px; flex: none">{mmss(s)}</span><span style="flex: 1 1 0">{t}</span></button>''')
    return f'''<div style="display: flex; flex-direction: column; gap: 8px">
<div style="color: #aeb6c4; font-size: 13px">Chapters</div>
<div style="background: #333944; border-radius: 9px; display: flex; flex-direction: column; overflow: hidden">{''.join(rows)}</div>
</div>'''
boards["AiPageChapters.dc.html"] = (
    "The page — chapters, A: listed under the player (not chosen: Bagrat picked B, 2026-10-18)", 1280, 1160,
    page("shroom video page, chapters under the player", 1280, 1160, web(
        viewer_head(TITLE) + "\n" + player(80, playing=True) + "\n" + chapter_list(80),
        VIEWER_FOOT,
        note("<b>Option A, not chosen</b> (Bagrat picked B, 2026-10-18). Kept for the record: the seek bar cut into chapters plus a list under the player."))))

# ───────── the Mac: Settings · Space and the Library ─────────

TRAFFIC = '<span style="width: 12px; height: 12px; border-radius: 6px; background: #4c566a; display: block"></span>' * 3
DONE = '<svg viewBox="0 0 16 16" role="img" aria-label="Done" style="width: 16px; height: 16px; fill: none; stroke: #a3be8c; stroke-width: 1.8; stroke-linecap: round; stroke-linejoin: round; display: block; flex: none"><path d="M3 8.5l3.2 3.2L13 4.8"></path></svg>'

def switch_row(label, label_id, line, line_id, on, sid):
    return f'''<div style="padding: 10px 12px; display: flex; flex-direction: row; align-items: center; gap: 10px">
<span style="flex: 1 1 0; display: flex; flex-direction: column">
<span id="{sid}" data-said="{label_id}">{label}</span>
<span style="color: #aeb6c4; font-size: 12px" data-said="{line_id}">{line}</span>
</span>
{(SW_ON if on else SW_OFF).format(id=sid)}
</div>'''

def settings_body(ai_on, ai_line, ai_line_id):
    rows = [("Cloudflare account", "Your Account"), ("Access token valid", "on this Mac"), ("Cloudflare billing set up", ""), ("Your link name", "v.myshroom.workers.dev"), ("Wired up", ""), ("macOS permissions granted", "on this Mac")]
    plan = "".join(f'''<div style="height: 40px; padding: 0 12px;{' border-top: 1px solid #434a56;' if i else ''} display: flex; flex-direction: row; align-items: center; gap: 10px">
{DONE}
<span style="flex: 1 1 0">{a}</span>{f'<span style="color: #aeb6c4">{b}</span>' if b else ''}
</div>''' for i, (a, b) in enumerate(rows))
    return f'''<div style="width: 600px; height: 760px; box-sizing: border-box; background: #2b2f36; display: flex; flex-direction: column; font-family: {FONT}; font-size: 13px; line-height: 1.5; color: #eceff4">
<div style="height: 44px; padding: 0 14px; display: flex; flex-direction: row; align-items: center; gap: 8px; flex: none">
{TRAFFIC}
<div style="flex: 1 1 0; display: flex; flex-direction: row; justify-content: center; padding: 0 52px 0 0"><div style="display: flex; flex-direction: row; background: #262a31; border-radius: 7px; padding: 2px; gap: 2px">
<button aria-pressed="false" style="font: inherit; border: 0; border-radius: 5px; background: transparent; color: #c3cad6; padding: 3px 16px" data-said="settings.general">General</button>
<button aria-pressed="true" style="font: inherit; border: 0; border-radius: 5px; background: #4c566a; color: #eceff4; padding: 3px 16px; font-weight: 600" data-said="settings.space">Space</button>
<button aria-pressed="false" style="font: inherit; border: 0; border-radius: 5px; background: transparent; color: #c3cad6; padding: 3px 16px" data-said="settings.members">Members</button>
</div></div></div>
<div style="flex: 1 1 0; padding: 16px 32px 28px 32px; display: flex; flex-direction: column; gap: 16px">
<div style="font-size: 14px; color: #c3cad6" data-said="account.your-space-set-up">Your space — set up once, yours forever:</div>
<div style="background: #333944; border-radius: 9px; display: flex; flex-direction: column">{plan}</div>
<div style="color: #aeb6c4; padding: 0 2px" data-said="settings-space-inspection.your-space-is-ready">Your space is ready — nothing to redo.</div>
<div style="background: #333944; border-radius: 9px; display: flex; flex-direction: column">
{switch_row("“made with shroom” on your pages", "settings-space.made-with-shroom-on", "The small mark on every video page. You can flip it right on a page, too.", "settings-space.the-small-mark-on", True, "mark-label")}
<div style="border-top: 1px solid #434a56">{switch_row(AI_LABEL, "settings-space.ai-for-new-videos", ai_line, ai_line_id, ai_on, "ai-label")}</div>
<button style="font: inherit; height: 40px; padding: 0 12px; border: 0; border-top: 1px solid #434a56; background: transparent; color: #e88790; text-align: left" data-said="settings-space.disconnect-this-mac">Disconnect this Mac…</button>
</div>
</div></div>'''

AI_LABEL = "AI for new videos"
AI_LINE = "Titles, chapters and captions from what’s said, and your “shroom cut” slips cut — on your own Cloudflare account. Free for about 2½ hours of new video a day; more is about 4¢ per video hour on Cloudflare’s $5 plan."
FAILED = "Couldn't change that just now — try again in a moment."
boards["AiSettings.dc.html"] = (
    "Settings — Space: AI for new videos, one switch for every AI feature, off as it ships (the Mac that holds the setup key; N66, approved as drawn, Bagrat 2026-10-18; he may tweak the words later)", 600, 760,
    page("shroom Settings, Space, AI", 600, 760, settings_body(False, AI_LINE, "settings-space.titles-chapters-and-captions")))
boards["AiSettingsFailed.dc.html"] = (
    "Settings — Space: AI couldn't change (the switch as the page has it, until the next try)", 600, 760,
    page("shroom Settings, Space, AI couldn't change", 600, 760, settings_body(False, FAILED, "settings-space.couldnt-change-that-just")))

def library_body(line):
    def row(t, meta, first=False):
        return f'''<div style="height: 50px; padding: 0 12px 0 10px;{'' if first else ' border-top: 1px solid #434a56;'} display: flex; flex-direction: row; align-items: center; gap: 10px">
<div style="width: 64px; height: 36px; border-radius: 5px; background: #262a31; display: flex; align-items: center; justify-content: center; flex: none"><svg viewBox="0 0 16 16" aria-hidden="true" style="width: 12px; height: 12px; fill: #7b8494; display: block"><path d="M5 3.5v9l7.5-4.5z"></path></svg></div>
<div style="flex: 1 1 0; min-width: 0; display: flex; flex-direction: column"><span style="overflow: hidden; text-overflow: ellipsis; white-space: nowrap">{t}</span><span style="color: #aeb6c4; font-size: 12px">{meta}</span></div>
<button style="font: inherit; height: 28px; padding: 0 12px; border: 1px solid #4c566a; border-radius: 7px; background: transparent; color: #eceff4; flex: none">Copy link</button></div>'''
    return f'''<div style="width: 760px; height: 560px; box-sizing: border-box; background: #2b2f36; display: flex; flex-direction: column; font-family: {FONT}; font-size: 13px; line-height: 1.45; color: #eceff4">
<div style="height: 36px; padding: 0 14px; display: flex; flex-direction: row; align-items: center; gap: 8px; flex: none">{TRAFFIC}</div>
<div style="flex: 1 1 0; padding: 4px 24px 16px 24px; display: flex; flex-direction: column; gap: 14px; min-height: 0">
<div style="display: flex; flex-direction: row; align-items: center; gap: 14px">
<h1 style="margin: 0; font-size: 18px; font-weight: 600; flex: 1 1 0">Library</h1>
<div style="width: 200px; height: 28px; box-sizing: border-box; padding: 0 8px; border: 1px solid #4c566a; border-radius: 8px; background: #262a31; display: flex; flex-direction: row; align-items: center; gap: 6px">
<svg viewBox="0 0 16 16" aria-hidden="true" style="width: 14px; height: 14px; fill: none; stroke: #aeb6c4; stroke-width: 1.5; stroke-linecap: round; display: block; flex: none"><circle cx="7" cy="7" r="4.2"></circle><path d="M10.2 10.2l3.3 3.3"></path></svg>
<input type="search" aria-label="Search titles" placeholder="Search titles" style="font: inherit; flex: 1 1 0; min-width: 0; border: 0; padding: 0; background: transparent; color: #eceff4">
</div>
</div>
<div role="group" aria-label="shroom can name your videos and add chapters" style="background: rgba(136, 192, 208, 0.10); border: 1px solid rgba(136, 192, 208, 0.35); border-radius: 9px; padding: 12px 14px; display: flex; flex-direction: row; align-items: center; gap: 14px">
<svg viewBox="0 0 16 16" aria-hidden="true" style="width: 20px; height: 20px; fill: none; stroke: #88c0d0; stroke-width: 1.3; stroke-linecap: round; stroke-linejoin: round; display: block; flex: none">{SPARK}</svg>
<div style="flex: 1 1 0; min-width: 0; display: flex; flex-direction: column; gap: 2px"><span style="font-weight: 600">shroom can name your videos and add chapters</span><span style="color: #d8dee9; font-size: 12px">{line}</span></div>
<button style="font: inherit; font-weight: 600; height: 28px; padding: 0 12px; border: 0; border-radius: 7px; background: #88c0d0; color: #2b2f36; white-space: nowrap; flex: none">Turn on</button>
<button style="font: inherit; border: 0; padding: 0 4px; background: transparent; color: #aeb6c4; white-space: nowrap; flex: none">Not now</button>
</div>
<div style="background: #333944; border-radius: 9px; display: flex; flex-direction: column; overflow: hidden">
{row("Where the 502s come from", "Oct 18 · 4:32", first=True)}
{row("Untitled — Oct 17", "Oct 17 · 0:48")}
{row("Onboarding flow, take 2", "Oct 16 · 12:10")}
</div>
<div style="flex: 1 1 0"></div>
<div style="color: #aeb6c4; font-size: 12px; text-align: center">3 videos</div>
</div></div>'''

boards["AiOffer.dc.html"] = (
    "Library — the naming card (replaced by the popover's banner, Bagrat 2026-10-18: too hidden; kept for the record)", 760, 560,
    page("shroom Library, naming card", 760, 560, library_body("From what’s said in them, by your own Cloudflare account. Free for about three hours of video a day; past that, about 4¢ an hour.")))
# ───────── the popover: the AI banner (Bagrat, 2026-10-18: visible, off by default) ─────────

MENU_MARK = MARK.replace('style="width: 17px; height: 18px; display: block"', 'role="img" aria-label="shroom" style="width: 16px; height: 17px; display: block"')

def popover_frame(panel, menu_extra=""):
    """The menu bar and the popover hanging from shroom's mark, as the Main and JustUpdated boards draw them."""
    return f'''<div style="width: 520px; height: 520px; box-sizing: border-box; background: #3b4252; display: flex; flex-direction: column; font-family: {FONT}; font-size: 13px; line-height: 1.45; color: #eceff4">
<div style="height: 28px; background: #2e3440; display: flex; flex-direction: row; justify-content: flex-end; align-items: center; padding: 0 86px 0 0; box-sizing: border-box">
<div style="width: 30px; height: 22px; border-radius: 5px; background: rgba(236, 239, 244, 0.14); display: flex; align-items: center; justify-content: center">{MENU_MARK}</div>
</div>
<div style="display: flex; flex-direction: row; justify-content: flex-end; padding: 8px 40px 0 0">
<div style="width: 300px; box-sizing: border-box; background: #2b2f36; border: 1px solid rgba(255, 255, 255, 0.1); border-radius: 12px; box-shadow: 0 8px 28px rgba(0, 0, 0, 0.3); padding: 12px 14px 14px 14px; display: flex; flex-direction: column; gap: 12px">
{panel}
</div>
</div>
{menu_extra}
</div>'''

TOOL = lambda label, d: f'<button aria-label="{label}" style="width: 28px; height: 28px; border: 0; background: transparent; color: #aeb6c4; padding: 0; border-radius: 6px; display: flex; align-items: center; justify-content: center"><svg viewBox="0 0 16 16" aria-hidden="true" style="width: 16px; height: 16px; fill: none; stroke: currentColor; stroke-width: 1.4; stroke-linecap: round; display: block">{d}</svg></button>'
HEADER = f'''<div style="display: flex; flex-direction: row; justify-content: space-between; align-items: center">
<div style="display: flex; flex-direction: row; align-items: center; gap: 7px">{MARK}<div style="font-size: 14px; font-weight: 600">shroom</div></div>
<div style="display: flex; flex-direction: row; gap: 2px">{TOOL("Library", '<path d="M4.5 2.5h7"></path><rect x="2" y="5" width="12" height="9" rx="1.8"></rect><path d="M7 7.8v3.4l2.8-1.7z" stroke-linejoin="round"></path>')}{TOOL("Settings", '<circle cx="8" cy="8" r="2"></circle><path stroke-linejoin="round" d="M12.54 6.16L14.46 6.63L14.46 9.37L12.54 9.84L11.86 11.02L12.42 12.9L10.04 14.28L8.68 12.85L7.32 12.85L5.96 14.28L3.58 12.9L4.14 11.02L3.46 9.84L1.54 9.37L1.54 6.63L3.46 6.16L4.14 4.98L3.58 3.1L5.96 1.72L7.32 3.15L8.68 3.15L10.04 1.72L12.42 3.1L11.86 4.98Z"></path>')}{TOOL("Quit shroom", '<path d="M8 2v6M4.5 4.5a5 5 0 1 0 7 0"></path>')}</div>
</div>'''
CHEVRON = '<svg viewBox="0 0 12 12" aria-hidden="true" style="width: 10px; height: 10px; fill: none; stroke: #aeb6c4; stroke-width: 1.5; stroke-linecap: round; stroke-linejoin: round; display: block; flex: none"><path d="M3 4.5l3 3 3-3"></path></svg>'
def pick(label, d, first=False):
    return f'''<div style="height: 38px; padding: 0 10px;{'' if first else ' border-top: 1px solid #434a56;'} display: flex; flex-direction: row; align-items: center; gap: 10px">
<svg viewBox="0 0 16 16" aria-hidden="true" style="width: 16px; height: 16px; fill: none; stroke: #aeb6c4; stroke-width: 1.4; stroke-linecap: round; stroke-linejoin: round; display: block; flex: none">{d}</svg>
<span style="flex: 1 1 0; min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap">{label}</span>{CHEVRON}</div>'''
CONFIG = f'''<div style="background: #333944; border-radius: 9px; display: flex; flex-direction: column">
{pick("Built-in Display", '<rect x="1.5" y="2.5" width="13" height="9" rx="1.5"></rect><path d="M6 14h4M8 11.5V14"></path>', first=True)}
{pick("MacBook Pro Microphone", '<rect x="6" y="1.5" width="4" height="8" rx="2"></rect><path d="M3.5 7.5a4.5 4.5 0 0 0 9 0M8 12v2.5"></path>')}
{pick("FaceTime HD Camera", '<rect x="1.5" y="4" width="9" height="8" rx="1.8"></rect><path d="M10.5 7l4-2v6l-4-2"></path>')}
</div>
<div style="display: flex; flex-direction: column; gap: 6px">
<button style="font: inherit; font-weight: 600; height: 36px; border: 0; border-radius: 8px; background: #88c0d0; color: #2b2f36" data-said="config.start-recording">Start recording</button>
<div style="color: #aeb6c4; font-size: 12px; text-align: center" data-said="config.starts-after-3-2">Starts after 3-2-1 — click anywhere to cancel.</div>
</div>'''

BANNER_LINE = "shroom can name your videos, add chapters and cut your slips for you."
def frost(line, line_id, buttons=True):
    """The frost banner, as the update's: a line beside a dot, Not now and Turn on… under it."""
    row = '''<div style="display: flex; flex-direction: row; align-items: center; gap: 10px; margin-left: auto">
<button style="font: inherit; border: 0; padding: 0; background: transparent; color: #c3cad6" data-said="ai-offer.not-now">Not now</button>
<button style="font: inherit; font-weight: 600; height: 28px; padding: 0 12px; border: 0; border-radius: 7px; background: #88c0d0; color: #2b2f36" data-said="ai-offer.turn-on">Turn on…</button>
</div>''' if buttons else ""
    return f'''<div style="background: #33424f; border-radius: 9px; padding: 8px 10px; display: flex; flex-direction: row; flex-wrap: wrap; align-items: center; gap: 8px">
<div style="flex: 1 1 140px; display: flex; flex-direction: row; align-items: center; gap: 8px">
<span style="width: 8px; height: 8px; border-radius: 4px; background: #88c0d0; display: block; flex: none"></span>
<div data-said="{line_id}">{line}</div>
</div>
{row}
</div>'''

boards["AiBanner.dc.html"] = (
    "Popover — the AI banner: shroom can name, chapter and cut your videos (the Mac that holds the setup key, while AI is off; N66, approved as drawn, Bagrat 2026-10-18; he may tweak the words later)", 520, 520,
    page("shroom popover, the AI banner", 520, 520, popover_frame(HEADER + "\n" + frost(BANNER_LINE, "ai-offer.shroom-can-name-your") + "\n" + CONFIG)))

CONFIRM_TITLE = "Turn on AI for new videos?"
boards["AiConfirm.dc.html"] = (
    "Popover — Turn on…: what it does and costs, once (N66, approved as drawn, Bagrat 2026-10-18; he may tweak the words later)", 520, 520,
    page("shroom popover, turn on AI", 520, 520, popover_frame(f'''<div style="display: flex; flex-direction: column; gap: 6px">
<h1 style="margin: 0; font-size: 15px; font-weight: 600; line-height: 1.35" data-said="ai-confirm.turn-on-ai-for">{CONFIRM_TITLE}</h1>
<div style="color: #c3cad6" data-said="ai-confirm.titles-chapters-and-captions">{AI_LINE}</div>
</div>
<div style="display: flex; flex-direction: row; justify-content: space-between; align-items: center; gap: 8px">
<button style="font: inherit; height: 32px; padding: 0 14px; border: 1px solid #4c566a; border-radius: 8px; background: transparent; color: #eceff4" data-said="ai-confirm.back">Back</button>
<button style="font: inherit; font-weight: 600; height: 32px; padding: 0 14px; border: 0; border-radius: 8px; background: #88c0d0; color: #2b2f36" data-said="ai-confirm.turn-on">Turn on</button>
</div>''')))

def states_body():
    def panel(label, inner):
        return f'''<div style="display: flex; flex-direction: column; gap: 8px"><div style="color: #aeb6c4; font-size: 12px">{label}</div>
<div style="width: 300px; box-sizing: border-box; background: #2b2f36; border: 1px solid rgba(255, 255, 255, 0.1); border-radius: 12px; padding: 12px 14px 14px 14px; display: flex; flex-direction: column; gap: 12px">{inner}</div></div>'''
    status = '''<div style="display: flex; flex-direction: row; align-items: center; gap: 7px; color: #aeb6c4; font-size: 12px">
<span style="width: 6px; height: 6px; border-radius: 3px; background: #a3be8c; display: block; flex: none"></span>
<div data-said="ai-on.ai-is-on-for">AI is on for new videos ✓</div>
</div>'''
    return f'''<div style="width: 520px; height: 900px; box-sizing: border-box; background: #3b4252; padding: 28px 0; display: flex; flex-direction: column; align-items: center; gap: 22px; font-family: {FONT}; font-size: 13px; line-height: 1.45; color: #eceff4">
{panel("After Turn on: the space's page is put up again with AI, a few seconds", frost("Turning on — a few seconds.", "ai-turning-on.turning-on-a-few", buttons=False))}
{panel("When it didn't go through: Turn on… is live again (law 12)", frost(FAILED, "ai-offer-failed.couldnt-change-that-just"))}
{panel("Once it's on: said once, where the update's “Updated ✓” goes", CONFIG + status)}
<div style="width: 300px">{note("Only on the Mac that holds the setup key, and only while AI is off; members' Macs never show it. It sits under the other banners (a saved take, the space, recording off, an update come first). <b>Not now</b> hides it for good on this Mac (Bagrat, 2026-10-18): from then on the switch in Settings · Space turns AI on. New videos only: earlier ones keep their titles (Bagrat, 2026-10-18).")}</div>
</div>'''
boards["AiBannerStates.dc.html"] = (
    "Popover — the AI banner's states: turning on, didn't go through, on (N66, approved as drawn, Bagrat 2026-10-18; he may tweak the words later)", 520, 900,
    page("shroom popover, the AI banner's states", 520, 900, states_body()))

# ───────── shroom.am: where AI's price sits (the agreed line; it goes up with the homepage's redesign) ─────────

def landing_body():
    return f'''<div style="width: 840px; height: 300px; box-sizing: border-box; background: #2b2f36; padding: 40px 48px; display: flex; flex-direction: column; gap: 14px; font-family: {FONT}; font-size: 16px; line-height: 1.5; color: #eceff4">
<div style="width: 744px; display: flex; flex-direction: column; gap: 8px">
<div style="position: relative; width: 100%; height: 6px; border-radius: 3px; background: #434a56"><div style="position: absolute; left: 0; top: 0; bottom: 0; width: 10%; border-radius: 3px 0 0 3px; background: #88c0d0"></div></div>
<div style="display: flex; flex-direction: row; justify-content: space-between; gap: 12px; font-size: 15px; color: #aeb6c4"><span><strong style="color: #88c0d0; font-weight: 600">$0</strong> · about three hours of video</span><span><strong style="color: #eceff4; font-weight: 600">$1.50 /month</strong> · about thirty hours</span></div>
<div style="display: flex; flex-direction: row; align-items: flex-start; gap: 8px; margin-top: 10px; font-size: 15px; color: #aeb6c4">
<svg viewBox="0 0 16 16" aria-hidden="true" style="width: 16px; height: 16px; fill: none; stroke: #88c0d0; stroke-width: 1.4; stroke-linecap: round; stroke-linejoin: round; display: block; flex: none; margin-top: 3px">{SPARK}</svg>
<span>AI titles, chapters and captions are free for about 2½ hours of new video a day. Recording more? About 4¢ per video hour on Cloudflare’s $5 plan.</span>
</div>
</div>
{note("The agreed line (Bagrat, 2026-10-18). It goes up with the homepage's redesign, not before; this board shows one place it could sit: under the price gauge, in the gauge's own grey.")}
</div>'''
boards["LandingAiLine.dc.html"] = (
    "shroom.am — the AI line, agreed (Bagrat, 2026-10-18; it goes up with the homepage's redesign)", 840, 300,
    page("shroom.am, the AI line", 840, 300, landing_body()))

# ───────── shroom.am/privacy: what the AI hears (approved as drafted, Bagrat 2026-10-18) ─────────

PRIVACY_AI = "runs in your own Cloudflare account: it hears what’s said in your new videos to name them and add chapters and captions. What it hears stays in your space and goes when you delete the video; it never reaches us."
PRIVACY_CUT = "When you say “shroom cut”, the slip is cut from what viewers get; the words cut stay in your space, and only you see them."

def privacy_body():
    def li(lead, rest):
        return f'<li><b style="color: #eceff4; font-weight: 600">{lead}</b> {rest}</li>'
    return f'''<div style="width: 760px; height: 560px; box-sizing: border-box; background: #2b2f36; padding: 36px 48px; display: flex; flex-direction: column; gap: 14px; font-family: {FONT}; font-size: 16px; line-height: 1.55; color: #eceff4">
<div style="font-size: 32px; font-weight: 700; line-height: 1.2">Privacy</div>
<div style="font-size: 18px; color: #c3cad6">shroom is built so that we hold nothing of yours.</div>
<ul style="margin: 0; padding-left: 22px; display: flex; flex-direction: column; gap: 10px; color: #c3cad6">
{li("Your videos", "live in your own Cloudflare account. We never receive them, store them or see them. Who can watch a video is up to you: anyone with its link can.")}
<li><div style="margin-left: -6px; padding: 8px 12px 8px 10px; border-left: 3px solid #88c0d0; border-radius: 0 9px 9px 0; background: rgba(136, 192, 208, 0.08)"><b style="color: #eceff4; font-weight: 600">AI, if you turn it on,</b> {PRIVACY_AI} <span style="color: #aeb6c4; font-style: italic">{PRIVACY_CUT}</span></div></li>
{li("The app", "talks to your Cloudflare account (to set it up, upload your videos and manage your space) and to GitHub (to check for updates, sending no ID or version) …")}
</ul>
{note("Approved as drafted (Bagrat, 2026-10-18) and built (site/pages/privacy.html): one new point, right after “Your videos”, in the page’s own words (true to the build: the space hears new videos only while AI is on, keeps the words at rec/&lt;id&gt;/ai/, and deleting a video clears everything under it). The grey sentence after it is the voice cuts’ line (N65, theirs), so the two read as one point. The rest of the page as it is; its date reads 18 October 2026.")}
</div>'''
boards["AiPrivacy.dc.html"] = (
    "shroom.am/privacy — what the AI hears (approved as drafted, Bagrat 2026-10-18)", 760, 560,
    page("shroom.am/privacy, the AI point", 760, 560, privacy_body()))

# One column right of the editor's, top to bottom: the Mac's boards, then the page's.
X = 11600
place = ["AiBanner.dc.html", "AiConfirm.dc.html", "AiBannerStates.dc.html", "AiSettings.dc.html", "AiSettingsFailed.dc.html", "AiOwnerNamed.dc.html", "AiNamingLater.dc.html",
         "AiPageMarks.dc.html", "AiPageMarksPhone.dc.html", "AiCaptions.dc.html", "LandingAiLine.dc.html", "AiPrivacy.dc.html", "AiOffer.dc.html", "AiPageChapters.dc.html"]
canvas_path = os.path.join(out, "canvas.json")
raw = open(canvas_path).read()
lines = raw.split("\n")
# Keep the file's own shape: its indent, or one line when it came as one.
indent = (len(lines[1]) - len(lines[1].lstrip()) or 1) if len(lines) > 2 else None
canvas = json.loads(raw)
y = 0
for name in place:
    title, w, h, html = boards[name]
    open(os.path.join(out, name), "w").write(html)
    canvas["boards"][name] = {"h": h, "radius": 12, "title": title, "w": w, "x": X, "y": y}
    if name not in canvas["order"]:
        canvas["order"].append(name)
    y += h + 100
json.dump(canvas, open(canvas_path, "w"), ensure_ascii=False, indent=indent)
print("wrote", ", ".join(place))
