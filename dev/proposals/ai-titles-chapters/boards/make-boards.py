#!/usr/bin/env python3
"""The AI titles and chapters boards for the shroom UI canvas (a proposal; words are drafts).

Each board is an x-dc page in the canvas's own style (Nord tokens, as RecipientLive,
SettingsSpace and LibraryRows are drawn). Usage: make-boards.py <canvas project dir>
writes the boards there and places them in canvas.json, in their own column."""
import json, sys, os

out = sys.argv[1]
FONT = "-apple-system, BlinkMacSystemFont, system-ui, sans-serif"
MARK = '''<svg viewBox="18 20 84 90" aria-hidden="true" style="width: 17px; height: 18px; display: block">
<path d="M50,68 C49,82 46,92 46,96 C46,101 52,104 60,104 C68,104 74,101 74,96 C74,92 71,82 70,68 Z" fill="#E5E9F0"></path>
<path d="M24,64 C24,42 40,26 60,26 C80,26 96,42 96,64 C86,75 34,75 24,64 Z" fill="#88C0D0"></path>
<circle cx="50" cy="44" r="6" fill="#ECEFF4"></circle><circle cx="72" cy="48" r="3.6" fill="#ECEFF4"></circle><circle cx="33" cy="58" r="3.4" fill="#ECEFF4"></circle><circle cx="64" cy="58" r="2.6" fill="#ECEFF4"></circle><circle cx="88" cy="58" r="2.2" fill="#ECEFF4"></circle>
</svg>'''
SW_ON = '<button role="switch" aria-checked="true" aria-labelledby="{id}" style="width: 36px; height: 20px; border: 0; border-radius: 10px; background: #88c0d0; padding: 2px; display: flex; flex-direction: row; justify-content: flex-end; align-items: center; flex: none"><span style="width: 16px; height: 16px; border-radius: 8px; background: #2b2f36; display: block"></span></button>'
SW_OFF = '<button role="switch" aria-checked="false" aria-labelledby="{id}" style="width: 36px; height: 20px; border: 0; border-radius: 10px; background: #4c566a; padding: 2px; display: flex; flex-direction: row; justify-content: flex-start; align-items: center; flex: none"><span style="width: 16px; height: 16px; border-radius: 8px; background: #d8dee9; display: block"></span></button>'

def page(title, w, h, body, bg=True):
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
    return f'<div style="margin-top: 14px; padding: 10px 12px; border: 1px dashed #4c566a; border-radius: 9px; color: #aeb6c4; font-size: 12px; line-height: 1.5">{text}</div>'

# A take from the POC's live run (invented script, Whisper turbo + gpt-oss-120b, 2026-10-18).
TITLE = "Sign up screen, invoice bug, trial numbers"
DUR = 183
CHAPTERS = [(0, "Sign up screen changes"), (68, "Invoice page bug details"), (114, "Trial results and metrics"), (157, "Next steps and wrap up")]
mmss = lambda t: f"{t // 60}:{t % 60:02d}"

def seekbar(now, marks, hover=None):
    """The seek bar, cut into one segment per chapter (2 px gaps), played part lit."""
    segs = []
    for i, (s, t) in enumerate(CHAPTERS):
        e = CHAPTERS[i + 1][0] if i + 1 < len(CHAPTERS) else DUR
        frac = (e - s) / DUR * 100
        played = max(0, min(e, now) - s) / (e - s) * 100
        segs.append(f'<div title="{t}" style="width: calc({frac:.2f}% - 2px); height: {6 if hover == i else 4}px; border-radius: 2px; background: linear-gradient(to right, #88c0d0 {played:.1f}%, #434a56 {played:.1f}%); flex: none"></div>')
    knob = now / DUR * 100
    tip = ""
    if hover is not None:
        s, t = CHAPTERS[hover]
        left = (s + 8) / DUR * 100
        tip = f'<div style="position: absolute; left: {left:.1f}%; bottom: 18px; transform: translateX(-50%); background: #1f2228; border: 1px solid #434a56; border-radius: 7px; padding: 4px 9px; white-space: nowrap; font-size: 12px; color: #eceff4"><span style="color: #aeb6c4; font-variant-numeric: tabular-nums">{mmss(s + 8)}</span>  {t}</div>'
    return f'''<div role="slider" tabindex="0" aria-label="Seek" aria-valuemin="0" aria-valuemax="{DUR}" aria-valuenow="{now}" aria-valuetext="{mmss(now)} of {mmss(DUR)}, {CHAPTERS[[i for i,(s,_) in enumerate(CHAPTERS) if s <= now][-1]][1]}" style="height: 16px; margin: -6px 0 -6px 0; display: flex; flex-direction: row; align-items: center; gap: 2px; position: relative">
{''.join(segs)}
<div style="position: absolute; left: calc({knob:.2f}% - 7px); width: 14px; height: 14px; border-radius: 7px; background: #eceff4"></div>
{tip}
</div>'''

def player(now, hover=None, chapter_label=True):
    cur = [t for s, t in CHAPTERS if s <= now][-1]
    label = f'<span style="color: #aeb6c4; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; min-width: 0">· {cur}</span>' if chapter_label else ''
    return f'''<div style="width: 100%; aspect-ratio: 16 / 9; border-radius: 12px; background: #1f2228; display: flex; flex-direction: column; overflow: hidden">
<div style="flex: 1 1 0; display: flex; align-items: center; justify-content: center"></div>
<div style="background: #262a31; display: flex; flex-direction: column; flex: none">
{seekbar(now, True, hover)}
<div style="height: 52px; padding: 0 10px; display: flex; flex-direction: row; align-items: center; gap: 4px; color: #d8dee9; font-size: 13px">
<button aria-label="Pause" style="width: 44px; height: 44px; border: 0; border-radius: 8px; background: transparent; display: flex; align-items: center; justify-content: center; flex: none">
<svg viewBox="0 0 16 16" aria-hidden="true" style="width: 18px; height: 18px; fill: #eceff4; display: block"><path d="M4 3h3v10H4zM9 3h3v10H9z"></path></svg>
</button>
<span style="font-variant-numeric: tabular-nums; padding: 0 6px; white-space: nowrap"><span style="color: #eceff4">{mmss(now)}</span> / {mmss(DUR)}</span>
{label}
<div style="flex: 1 1 0"></div>
<button aria-label="Playback speed, 1×" style="font: inherit; height: 44px; min-width: 44px; padding: 0 10px; border: 0; border-radius: 8px; background: transparent; color: #d8dee9; font-weight: 600; flex: none">1×</button>
<button aria-label="Full screen" style="width: 44px; height: 44px; border: 0; border-radius: 8px; background: transparent; display: flex; align-items: center; justify-content: center; flex: none">
<svg viewBox="0 0 16 16" aria-hidden="true" style="width: 18px; height: 18px; fill: none; stroke: #d8dee9; stroke-width: 1.4; stroke-linecap: round; stroke-linejoin: round; display: block"><path d="M2.5 6V3.5H5M11 3.5h2.5V6M13.5 10v2.5H11M5 12.5H2.5V10"></path></svg>
</button>
</div>
</div>
</div>'''

def chapter_list(now):
    rows = []
    for i, (s, t) in enumerate(CHAPTERS):
        e = CHAPTERS[i + 1][0] if i + 1 < len(CHAPTERS) else DUR
        on = s <= now < e
        rows.append(f'''<button aria-current="{'true' if on else 'false'}" style="font: inherit; min-height: 44px; padding: 0 12px; border: 0; {'border-top: 1px solid #434a56; ' if i else ''}background: {'rgba(136, 192, 208, 0.12)' if on else 'transparent'}; color: #eceff4; display: flex; flex-direction: row; align-items: center; gap: 14px; text-align: left">
<span style="font-variant-numeric: tabular-nums; color: {'#88c0d0' if on else '#aeb6c4'}; width: 36px; flex: none">{mmss(s)}</span><span style="flex: 1 1 0">{t}</span></button>''')
    return f'''<div style="display: flex; flex-direction: column; gap: 8px">
<div style="color: #aeb6c4; font-size: 13px" data-said="page.chapters">Chapters</div>
<div style="background: #333944; border-radius: 9px; display: flex; flex-direction: column; overflow: hidden">{''.join(rows)}</div>
</div>'''

def video_page(h1, inner_after, now, hover=None, owner_title=None, chapter_label=True, extra_note=""):
    if owner_title:
        head = f'''<div style="display: flex; flex-direction: column; gap: 2px">
<div style="display: flex; flex-direction: row; align-items: center; gap: 4px">
<input type="text" aria-label="Title" value="{owner_title}" style="font: inherit; font-size: 1.75rem; font-weight: 600; flex: 1 1 0; min-width: 0; box-sizing: border-box; height: 52px; padding: 0; border: 1px solid transparent; border-radius: 9px; background: transparent; color: #eceff4">
<button aria-label="Copy link" style="width: 44px; height: 44px; border: 0; border-radius: 9px; background: transparent; display: flex; align-items: center; justify-content: center; flex: none"><svg viewBox="0 0 16 16" aria-hidden="true" style="width: 18px; height: 18px; fill: none; stroke: #c3cad6; stroke-width: 1.4; stroke-linecap: round; stroke-linejoin: round; display: block"><path d="M6.8 9.2a2.8 2.8 0 0 0 4 0l2.2-2.2a2.8 2.8 0 0 0-4-4l-.8.8M9.2 6.8a2.8 2.8 0 0 0-4 0L3 9a2.8 2.8 0 0 0 4 4l.8-.8"></path></svg></button>
</div>
<div style="color: #aeb6c4; font-size: 13px; display: flex; flex-direction: row; align-items: center; gap: 6px"><svg viewBox="0 0 16 16" aria-hidden="true" style="width: 14px; height: 14px; fill: none; stroke: #88c0d0; stroke-width: 1.4; stroke-linecap: round; stroke-linejoin: round; display: block"><path d="M8 2.5l1.3 3.2 3.2 1.3-3.2 1.3L8 11.5 6.7 8.3 3.5 7l3.2-1.3z"></path></svg><span data-said="page-owner.named-from-what-you-said">Named from what you said. Click the title to change it.</span></div>
</div>'''
    else:
        head = f'<div style="display: flex; flex-direction: row; align-items: center; gap: 4px; min-height: 52px"><h1 style="margin: 0; font-size: 1.75rem; font-weight: 600; line-height: 1.25; flex: 1 1 0; min-width: 0">{h1}</h1></div>'
    return f'''<div style="min-height: 100vh; box-sizing: border-box; background: #2b2f36; padding: 40px 16px 24px 16px; display: flex; flex-direction: column; align-items: center; font-family: {FONT}; font-size: 14px; line-height: 1.5; color: #eceff4">
<div style="width: 100%; max-width: 960px; flex: 1 1 auto; display: flex; flex-direction: column; gap: 20px">
{head}
{player(now, hover, chapter_label)}
{inner_after}
{extra_note}
</div>
<div style="width: 100%; max-width: 960px; padding: 32px 0 0 0; display: flex; flex-direction: row; justify-content: flex-end">
<a href="https://shroom.am" style="display: flex; flex-direction: row; align-items: center; gap: 8px; font-size: 13px; text-decoration: none; min-height: 44px">{MARK}<span data-said="live.made-with-shroom">made with shroom</span></a>
</div>
</div>'''

boards = {}

# 1. The page, chapters listed under the player and cut into its bar (option A).
boards["AiPageChapters.dc.html"] = (
    "The page — chapters, A: listed under the player and cut into its bar (a proposal, for Bagrat; words are drafts)", 1280, 1160,
    page("shroom video page, chapters under the player", 1280, 1160, video_page(TITLE, chapter_list(80), 80,
        extra_note=note("<b>Option A.</b> The seek bar is cut into one piece per chapter; the chapter you're in is lit in the list and named beside the time. A click on a row jumps there. Shown only when the video has 2 or more chapters (90 s or longer). The title and chapters here are the POC's real output on an invented 3-minute take."))))

# 2. The page, chapters only in the bar, named on hover (option B).
boards["AiPageMarks.dc.html"] = (
    "The page — chapters, B: only in the bar, named on hover (a proposal, for Bagrat)", 1280, 900,
    page("shroom video page, chapters in the bar only", 1280, 900, video_page(TITLE, "", 80, hover=2,
        extra_note=note("<b>Option B.</b> Nothing under the player: the bar's pieces are the chapters, and hovering one names it (a tap on a phone). Quieter, but a viewer can't see the list at a glance, and on a phone the names are hard to reach. The chapter you're in is named beside the time, as in A."))))

# 3. The owner's page once the title has come in.
boards["AiOwnerNamed.dc.html"] = (
    "The page — the owner's, named for you (a proposal, for Bagrat; words are drafts)", 1280, 1160,
    page("shroom video page, owner, named", 1280, 1160, video_page(TITLE, chapter_list(0), 0, owner_title=TITLE,
        extra_note=note("About 10–15 s after Stop, the title and chapters arrive (the take is heard while it records). Until then the title is today's (the date). A title the owner or member types always wins and is never written over; the line goes once the title is edited. Chapters can't be edited in v-next (an open question)."))))

# 4. Settings · Space: the switch and its one cost line.
def settings_body():
    tabs = '''<div style="height: 44px; padding: 0 14px; display: flex; flex-direction: row; align-items: center; gap: 8px; flex: none">
<span style="width: 12px; height: 12px; border-radius: 6px; background: #4c566a; display: block"></span><span style="width: 12px; height: 12px; border-radius: 6px; background: #4c566a; display: block"></span><span style="width: 12px; height: 12px; border-radius: 6px; background: #4c566a; display: block"></span>
<div style="flex: 1 1 0; display: flex; flex-direction: row; justify-content: center; padding: 0 52px 0 0"><div style="display: flex; flex-direction: row; background: #262a31; border-radius: 7px; padding: 2px; gap: 2px">
<button aria-pressed="false" style="font: inherit; border: 0; border-radius: 5px; background: transparent; color: #c3cad6; padding: 3px 16px" data-said="settings.general">General</button>
<button aria-pressed="true" style="font: inherit; border: 0; border-radius: 5px; background: #4c566a; color: #eceff4; padding: 3px 16px; font-weight: 600" data-said="settings.space">Space</button>
<button aria-pressed="false" style="font: inherit; border: 0; border-radius: 5px; background: transparent; color: #c3cad6; padding: 3px 16px" data-said="settings.members">Members</button>
</div></div></div>'''
    card = lambda on, sub, rows: f'''<div style="background: #333944; border-radius: 9px; display: flex; flex-direction: column">
<div style="padding: 10px 12px; display: flex; flex-direction: row; align-items: center; gap: 10px">
<span style="flex: 1 1 0; display: flex; flex-direction: column"><span id="ai-{on}" data-said="settings-space.name-videos">Name videos and add chapters</span><span style="color: #aeb6c4; font-size: 12px">{sub}</span></span>
{(SW_ON if on else SW_OFF).format(id=f"ai-{on}")}
</div>{rows}</div>'''
    row = lambda a, b: f'<div style="height: 40px; padding: 0 12px; border-top: 1px solid #434a56; display: flex; flex-direction: row; align-items: center; gap: 10px"><span style="flex: 1 1 0">{a}</span><span style="color: #aeb6c4">{b}</span></div>'
    sub = "From what's said in each video, by your own Cloudflare account. Everyone's videos in the space. A title someone types always wins."
    return f'''<div style="width: 600px; height: 700px; box-sizing: border-box; background: #2b2f36; display: flex; flex-direction: column; font-family: {FONT}; font-size: 13px; line-height: 1.5; color: #eceff4">
{tabs}
<div style="flex: 1 1 0; padding: 16px 32px 28px 32px; display: flex; flex-direction: column; gap: 12px">
<div style="color: #aeb6c4; font-size: 12px; padding: 0 2px">Off (as it ships)</div>
{card(False, sub + " Free for about 3 hours of video a day; past that, about 4¢ an hour.", "")}
<div style="color: #aeb6c4; font-size: 12px; padding: 12px 2px 0 2px">On</div>
{card(True, sub, row("This month", "11 h 20 min of video — free") + row("Past the free hours", "about 4¢ an hour · <a href='#'>Set a monthly limit…</a>"))}
{note("Owner only: members' Settings don't show it, and members never see a cost. On a free Cloudflare plan the free hours are a daily cap: past it, naming waits until the next day (00:00 UTC) and catches up. The monthly limit is an open question (only matters on Cloudflare's paid plan).")}
</div></div>'''
boards["AiSettings.dc.html"] = ("Settings — Space: Name videos and add chapters, off and on (a proposal, for Bagrat; words are drafts)", 600, 700, page("shroom Settings, Space, AI naming", 600, 700, settings_body()))

# 5. The one-time offer, on the owner's Library.
def offer_body():
    row = lambda t, meta: f'''<div style="height: 50px; padding: 0 12px 0 10px; border-top: 1px solid #434a56; display: flex; flex-direction: row; align-items: center; gap: 10px">
<div style="width: 64px; height: 36px; border-radius: 5px; background: #262a31; display: flex; align-items: center; justify-content: center; flex: none"><svg viewBox="0 0 16 16" aria-hidden="true" style="width: 12px; height: 12px; fill: #7b8494; display: block"><path d="M5 3.5v9l7.5-4.5z"></path></svg></div>
<div style="flex: 1 1 0; min-width: 0; display: flex; flex-direction: column"><span style="overflow: hidden; text-overflow: ellipsis; white-space: nowrap">{t}</span><span style="color: #aeb6c4; font-size: 12px">{meta}</span></div>
<button style="font: inherit; height: 28px; padding: 0 12px; border: 1px solid #4c566a; border-radius: 7px; background: transparent; color: #eceff4; flex: none">Copy link</button></div>'''
    return f'''<div style="width: 760px; height: 560px; box-sizing: border-box; background: #2b2f36; display: flex; flex-direction: column; font-family: {FONT}; font-size: 13px; line-height: 1.45; color: #eceff4">
<div style="height: 36px; padding: 0 14px; display: flex; flex-direction: row; align-items: center; gap: 8px; flex: none"><span style="width: 12px; height: 12px; border-radius: 6px; background: #4c566a; display: block"></span><span style="width: 12px; height: 12px; border-radius: 6px; background: #4c566a; display: block"></span><span style="width: 12px; height: 12px; border-radius: 6px; background: #4c566a; display: block"></span></div>
<div style="flex: 1 1 0; padding: 4px 24px 16px 24px; display: flex; flex-direction: column; gap: 14px; min-height: 0">
<h1 style="margin: 0; font-size: 18px; font-weight: 600">Library</h1>
<div style="background: rgba(136, 192, 208, 0.10); border: 1px solid rgba(136, 192, 208, 0.35); border-radius: 9px; padding: 12px 14px; display: flex; flex-direction: row; align-items: center; gap: 14px">
<svg viewBox="0 0 16 16" aria-hidden="true" style="width: 20px; height: 20px; fill: none; stroke: #88c0d0; stroke-width: 1.3; stroke-linecap: round; stroke-linejoin: round; display: block; flex: none"><path d="M8 2.5l1.3 3.2 3.2 1.3-3.2 1.3L8 11.5 6.7 8.3 3.5 7l3.2-1.3z"></path></svg>
<div style="flex: 1 1 0; display: flex; flex-direction: column; gap: 2px"><span style="font-weight: 600" data-said="library.ai-offer">shroom can name your videos and add chapters</span><span style="color: #c3cad6; font-size: 12px" data-said="library.ai-offer-cost">From what's said in them, by your own Cloudflare account. Free for about 3 hours of video a day; past that, about 4¢ an hour.</span></div>
<button style="font: inherit; font-weight: 600; height: 28px; padding: 0 12px; border: 0; border-radius: 7px; background: #88c0d0; color: #2b2f36; flex: none">Turn on</button>
<button style="font: inherit; border: 0; padding: 0 4px; background: transparent; color: #c3cad6; flex: none">Not now</button>
</div>
<div style="background: #333944; border-radius: 9px; display: flex; flex-direction: column; overflow: hidden">
{row("Oct 18, 9:40 am", "Oct 18 · 3:03").replace("border-top: 1px solid #434a56; ", "", 1)}
{row("Oct 17, 4:12 pm", "Oct 17 · 12:40")}
{row("Oct 16, 10:05 am", "Oct 16 · 0:52")}
</div>
{note("Once, on the owner's Library only (members never see it). Turn on is all it takes: the space's Worker is redeployed with AI, no new step in Cloudflare (B171, live check 2). Not now hides it; the switch stays in Settings · Space. Videos made before Turn on keep their titles (an open question: offer to name past videos?).")}
</div></div>'''
boards["AiOffer.dc.html"] = ("Library — the one-time offer: name your videos and add chapters (a proposal, for Bagrat; words are drafts)", 760, 560, page("shroom Library, AI offer", 760, 560, offer_body()))

X = 11600
place = [("AiOffer.dc.html", 0), ("AiSettings.dc.html", 700), ("AiOwnerNamed.dc.html", 1540), ("AiPageChapters.dc.html", 2840), ("AiPageMarks.dc.html", 4140)]
canvas_path = os.path.join(out, "canvas.json")
canvas = json.load(open(canvas_path))
for name, y in place:
    title, w, h, html = boards[name]
    open(os.path.join(out, name), "w").write(html)
    canvas["boards"][name] = {"h": h, "radius": 12, "title": title, "w": w, "x": X, "y": y}
    if name not in canvas["order"]:
        canvas["order"].append(name)
json.dump(canvas, open(canvas_path, "w"), ensure_ascii=False, indent=1)
print("wrote", ", ".join(n for n, _ in place))
