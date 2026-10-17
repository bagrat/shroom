#!/usr/bin/env python3
"""Writes the three in-app feedback proposal boards (A, B, C) as canvas boards.

The boards share their pieces (the popover, a window's chrome, the form), so they're
written from one place. Run it, then `node render.mjs` beside it for the PNGs.
"""
from pathlib import Path

OUT = Path(__file__).parent

MUSH = ('<svg viewBox="18 20 84 90" aria-hidden="true" style="width: {w}px; height: {h}px; display: block">'
        '<path d="M50,68 C49,82 46,92 46,96 C46,101 52,104 60,104 C68,104 74,101 74,96 C74,92 71,82 70,68 Z" fill="#E5E9F0"></path>'
        '<path d="M24,64 C24,42 40,26 60,26 C80,26 96,42 96,64 C86,75 34,75 24,64 Z" fill="#88C0D0"></path>'
        '<circle cx="33" cy="58" r="3.4" fill="#ECEFF4"></circle><circle cx="88" cy="58" r="2.2" fill="#ECEFF4"></circle>'
        '<circle cx="50" cy="44" r="6" fill="#ECEFF4"></circle><circle cx="72" cy="48" r="3.6" fill="#ECEFF4"></circle>'
        '<circle cx="64" cy="58" r="2.6" fill="#ECEFF4"></circle></svg>')

ICON = {
    "feedback": '<path d="M3 3h10a1.5 1.5 0 0 1 1.5 1.5v5.5a1.5 1.5 0 0 1-1.5 1.5H7.5L4.5 14v-2.5H3A1.5 1.5 0 0 1 1.5 10V4.5A1.5 1.5 0 0 1 3 3z" stroke-linejoin="round"></path>',
    "library": '<path d="M4.5 2.5h7"></path><rect x="2" y="5" width="12" height="9" rx="1.8"></rect><path d="M7 7.8v3.4l2.8-1.7z" stroke-linejoin="round"></path>',
    "settings": '<circle cx="8" cy="8" r="2"></circle><path stroke-linejoin="round" d="M12.54 6.16L14.46 6.63L14.46 9.37L12.54 9.84L11.86 11.02L12.42 12.9L10.04 14.28L8.68 12.85L7.32 12.85L5.96 14.28L3.58 12.9L4.14 11.02L3.46 9.84L1.54 9.37L1.54 6.63L3.46 6.16L4.14 4.98L3.58 3.1L5.96 1.72L7.32 3.15L8.68 3.15L10.04 1.72L12.42 3.1L11.86 4.98Z"></path>',
    "quit": '<path d="M8 2v6M4.5 4.5a5 5 0 1 0 7 0"></path>',
}
LABEL = {"feedback": "Send feedback", "library": "Library", "settings": "Settings", "quit": "Quit shroom"}


def glyph(name, color="currentColor", size=16):
    return (f'<svg viewBox="0 0 16 16" aria-hidden="true" style="width: {size}px; height: {size}px; fill: none; '
            f'stroke: {color}; stroke-width: 1.4; stroke-linecap: round; display: block">{ICON[name]}</svg>')


def icon_btn(name, hot=False):
    bg = "rgba(136, 192, 208, 0.16)" if hot else "transparent"
    col = "#88c0d0" if hot else "#aeb6c4"
    return (f'<button aria-label="{LABEL[name]}" title="{LABEL[name]}" style="width: 28px; height: 28px; border: 0; background: {bg}; '
            f'color: {col}; padding: 0; border-radius: 6px; display: flex; align-items: center; justify-content: center">{glyph(name)}</button>')


def tooltip(text):
    return (f'<div style="position: absolute; top: 34px; left: -28px; white-space: nowrap; background: #1a1d22; color: #eceff4; '
            f'font-size: 12px; padding: 4px 8px; border-radius: 6px; border: 1px solid #434a56">{text}</div>')


def header(icons, hot=None, tip=None):
    btns = []
    for n in icons:
        b = icon_btn(n, n == hot)
        if n == hot and tip:
            b = f'<div style="position: relative">{b}{tooltip(tip)}</div>'
        btns.append(b)
    return ('<div style="display: flex; flex-direction: row; justify-content: space-between; align-items: center">'
            f'<div style="display: flex; flex-direction: row; align-items: center; gap: 7px">{MUSH.format(w=17, h=18)}'
            '<div style="font-size: 14px; font-weight: 600">shroom</div></div>'
            f'<div style="display: flex; flex-direction: row; gap: 2px">{"".join(btns)}</div></div>')


def chevron():
    return ('<svg viewBox="0 0 12 12" aria-hidden="true" style="width: 10px; height: 10px; fill: none; stroke: #aeb6c4; stroke-width: 1.5; '
            'stroke-linecap: round; stroke-linejoin: round; display: block; flex: none"><path d="M3 4.5l3 3 3-3"></path></svg>')


def row(path, text, first=False):
    border = "" if first else "border-top: 1px solid #434a56; "
    return (f'<div style="height: 38px; padding: 0 10px; {border}display: flex; flex-direction: row; align-items: center; gap: 10px">'
            f'<svg viewBox="0 0 16 16" aria-hidden="true" style="width: 16px; height: 16px; fill: none; stroke: #aeb6c4; stroke-width: 1.4; '
            f'stroke-linecap: round; stroke-linejoin: round; display: block; flex: none">{path}</svg>'
            f'<span style="flex: 1 1 0; white-space: nowrap; overflow: hidden; text-overflow: ellipsis">{text}</span>{chevron()}</div>')


CARD = ('<div style="background: #333944; border-radius: 9px; display: flex; flex-direction: column">'
        + row('<rect x="1.5" y="2.5" width="13" height="9" rx="1.5"></rect><path d="M6 14h4M8 11.5V14"></path>', "Built-in Display", True)
        + row('<rect x="6" y="1.5" width="4" height="8" rx="2"></rect><path d="M3.5 7.5a4.5 4.5 0 0 0 9 0M8 12v2.5"></path>', "MacBook Pro Microphone")
        + row('<rect x="1.5" y="4" width="9" height="8" rx="1.8"></rect><path d="M10.5 7l4-2v6l-4-2"></path>', "FaceTime HD Camera")
        + '</div>')

START = ('<div style="display: flex; flex-direction: column; gap: 6px">'
         '<button style="font: inherit; font-weight: 600; height: 36px; border: 0; border-radius: 8px; background: #88c0d0; color: #2b2f36">Start recording</button>'
         '<div style="color: #aeb6c4; font-size: 12px; text-align: center">Starts after 3-2-1 — click anywhere to cancel.</div></div>')


def status_line(text, dot="#88c0d0", said=""):
    s = f' data-said="{said}"' if said else ""
    return ('<div style="display: flex; flex-direction: row; align-items: flex-start; gap: 7px; color: #aeb6c4; font-size: 12px">'
            f'<span style="width: 6px; height: 6px; border-radius: 3px; background: {dot}; display: block; flex: none; margin-top: 6px"></span>'
            f'<div{s}>{text}</div></div>')


def menubar():
    return ('<div style="height: 28px; background: #2e3440; display: flex; flex-direction: row; justify-content: flex-end; align-items: center; '
            'padding: 0 86px 0 0; box-sizing: border-box; flex: none"><div style="width: 30px; height: 22px; border-radius: 5px; '
            f'background: rgba(236, 239, 244, 0.14); display: flex; align-items: center; justify-content: center">{MUSH.format(w=16, h=17)}</div></div>')


def popover(inner, h=460):
    return (f'<div style="width: 520px; height: {h}px; box-sizing: border-box; background: #3b4252; border-radius: 10px; overflow: hidden; '
            f'display: flex; flex-direction: column; flex: none">{menubar()}'
            '<div style="display: flex; flex-direction: row; justify-content: flex-end; padding: 8px 40px 0 0">'
            '<div style="width: 300px; box-sizing: border-box; background: #2b2f36; border: 1px solid rgba(255, 255, 255, 0.1); border-radius: 12px; '
            f'box-shadow: 0 8px 28px rgba(0, 0, 0, 0.3); padding: 12px 14px 14px 14px; display: flex; flex-direction: column; gap: 12px">{inner}</div></div></div>')


def config_popover(hot=None, tip=None, extra="", icons=("feedback", "library", "settings", "quit")):
    return popover(header(icons, hot, tip) + CARD + START + extra)


def window(title, body, w=520, h=600, dim=False):
    light = "#3b4252" if dim else "#4c566a"
    return (f'<div style="width: {w}px; height: {h}px; box-sizing: border-box; background: #2b2f36; border-radius: 10px; '
            'border: 1px solid rgba(255, 255, 255, 0.08); display: flex; flex-direction: column; overflow: hidden; flex: none">'
            '<div style="height: 40px; padding: 0 14px; display: flex; flex-direction: row; align-items: center; gap: 8px; flex: none">'
            + "".join(f'<span style="width: 12px; height: 12px; border-radius: 6px; background: {light}; display: block"></span>' for _ in range(3))
            + f'<div style="flex: 1 1 0; text-align: center; padding: 0 52px 0 0; color: #aeb6c4; font-weight: 600">{title}</div></div>'
            f'<div style="flex: 1 1 0; padding: 4px 24px 22px 24px; display: flex; flex-direction: column; gap: 12px">{body}</div></div>')


FIELD = "font: inherit; box-sizing: border-box; border: 1px solid #4c566a; border-radius: 8px; background: #262a31; color: #eceff4"
BTN = "font: inherit; height: 32px; padding: 0 14px; border: 1px solid #4c566a; border-radius: 8px; background: transparent; color: #eceff4"
FILLED = "font: inherit; font-weight: 600; height: 32px; padding: 0 16px; border: 0; border-radius: 8px; background: #88c0d0; color: #2b2f36"
GREY = "font: inherit; font-weight: 600; height: 32px; padding: 0 16px; border: 0; border-radius: 8px; background: #4c566a; color: #7b8494"

WORDS = "Love it. One thing: I'd like the link copied the moment I stop, without opening the popover again."
PREVIEW = """Feedback, from the app
Love it. One thing: I'd like the link copied the moment…

shroom 0.1.18 · macOS 15.3.1 · Apple M2 · 16 GB
Set up 2026-10-12, 10:02–10:15: account · key · card · permissions · wired

— the last 200 lines of shroom's log, cleaned —
2026-10-17 09:12  boot: launch login · shroom 0.1.18
    1840 core config [Probe, Draw]
    2210 library refreshed (Owner): 0 new, 1 changed, 0 gone
   64120 core counting [Warm]
  …"""


def check(on, label, said, note=None):
    box = ('<span style="width: 16px; height: 16px; border-radius: 4px; flex: none; display: flex; align-items: center; justify-content: center; '
           + ('background: #88c0d0">' '<svg viewBox="0 0 12 12" style="width: 11px; height: 11px; fill: none; stroke: #2b2f36; stroke-width: 2; stroke-linecap: round; stroke-linejoin: round"><path d="M2.5 6.2l2.4 2.3 4.6-5"></path></svg>'
              if on else 'border: 1px solid #7b8494">')
           + '</span>')
    n = f'<div style="color: #aeb6c4; font-size: 12px; padding: 0 0 0 24px" data-said="{said}-note">{note}</div>' if note else ""
    return (f'<div style="display: flex; flex-direction: column; gap: 3px"><label style="display: flex; flex-direction: row; align-items: center; gap: 8px">'
            f'{box}<span data-said="{said}">{label}</span></label>{n}</div>')


def disclosure(open_=False):
    arrow = "M4.5 3l3 3-3 3" if not open_ else "M3 4.5l3 3 3-3"
    d = ('<button aria-expanded="%s" style="font: inherit; border: 0; background: transparent; color: #aeb6c4; padding: 0; display: flex; '
         'flex-direction: row; align-items: center; gap: 6px; align-self: flex-start" data-said="report-problem.see-whats-sent">'
         '<svg viewBox="0 0 12 12" style="width: 10px; height: 10px; fill: none; stroke: #aeb6c4; stroke-width: 1.5; stroke-linecap: round; '
         'stroke-linejoin: round"><path d="%s"></path></svg>See exactly what’s sent</button>') % ("true" if open_ else "false", arrow)
    if open_:
        d += ('<pre style="margin: 0; height: 120px; overflow: hidden; box-sizing: border-box; padding: 9px 11px; background: #262a31; '
              'border: 1px solid #434a56; border-radius: 8px; font-family: ui-monospace, SFMono-Regular, Menlo, monospace; font-size: 10.5px; '
              f'line-height: 1.5; color: #c3cad6; white-space: pre">{PREVIEW}</pre>')
    return d


def form(*, title, line, placeholder, words="", mail="", log=True, log_note=True, open_=False, sending=False, kind=None,
         cancel="Cancel", fixed_log=None):
    out = []
    if title:
        out.append(f'<h2 style="margin: 0; font-size: 15px; font-weight: 600" data-said="send-feedback.title">{title}</h2>')
    if kind:
        out.append(kind)
    out.append(f'<p style="margin: 0; color: #c3cad6" data-said="send-feedback.line">{line}</p>')
    val = words or ""
    ph_col = "#eceff4" if words else "#7b8494"
    out.append(f'<div style="{FIELD}; height: 84px; padding: 8px 10px; color: {ph_col}" data-said="send-feedback.placeholder">{val or placeholder}</div>')
    out.append('<div style="display: flex; flex-direction: column; gap: 4px"><label style="color: #c3cad6" data-said="report-problem.reply-to">'
               'Your email, if you’d like an answer (optional)</label>'
               f'<div style="{FIELD}; height: 30px; padding: 0 10px; display: flex; align-items: center">{mail}</div></div>')
    if fixed_log:
        out.append(f'<div style="color: #aeb6c4; font-size: 12px">{fixed_log}</div>')
    else:
        out.append(check(log, "Include shroom’s recent log", "send-feedback.include-log",
                         "Its version, your Mac’s model and its recent log, cleaned — never your videos, links, names or keys." if log_note else None))
    out.append(disclosure(open_))
    send = f'<button style="{GREY}" data-said="report-problem.sending">Sending…</button>' if sending else \
        f'<button style="{FILLED if words else GREY}" data-said="send-feedback.send">Send</button>'
    out.append('<div style="flex: 1 1 0"></div>')
    out.append(f'<div style="display: flex; flex-direction: row; align-items: center; gap: 8px"><span style="flex: 1 1 0"></span>'
               f'<button style="{BTN}">{cancel}</button>{send}</div>')
    return "".join(out)


SENT_TITLE = "Sent — thank you."
SENT_LINE = "If you left your email, you’ll hear back."


def sent_body():
    return ('<div style="flex: 1 1 0; display: flex; flex-direction: column; align-items: center; justify-content: center; gap: 10px; text-align: center">'
            '<div style="width: 44px; height: 44px; border-radius: 22px; background: rgba(136, 192, 208, 0.16); display: flex; align-items: center; justify-content: center">'
            '<svg viewBox="0 0 16 16" style="width: 22px; height: 22px; fill: none; stroke: #88c0d0; stroke-width: 1.6; stroke-linecap: round; stroke-linejoin: round"><path d="M3.5 8.4l3 2.9 6-6.6"></path></svg></div>'
            f'<div style="font-size: 15px; font-weight: 600" data-said="send-feedback.sent">{SENT_TITLE}</div>'
            f'<div style="color: #c3cad6" data-said="send-feedback.sent-line">{SENT_LINE}</div></div>'
            f'<div style="display: flex; flex-direction: row; justify-content: flex-end"><button style="{FILLED}" data-said="send-feedback.done">Done</button></div>')


def inbox(subject, sent_from, log=True):
    body = [f"From: you@example.com (reply-to)", "", "What they said:", WORDS, "",
            "shroom 0.1.18 · macOS 15.3.1 · Apple M2 · 16 GB", "Set up 2026-10-12, 10:02–10:15", ""]
    body += (["— the last 200 lines of shroom's log, cleaned —", "2026-10-17 09:12  boot: launch login · shroom 0.1.18", "…", ""]
             if log else ["(no log: they left it out)", ""])
    body.append(f"Sent from: {sent_from}")
    return ('<div style="width: 520px; box-sizing: border-box; background: #eceff4; color: #2e3440; border-radius: 10px; padding: 16px 18px; '
            'display: flex; flex-direction: column; gap: 8px; flex: none; font-size: 12.5px">'
            '<div style="color: #4c566a; font-size: 12px">report-sender@shroom.am → reports@shroom.am</div>'
            f'<div style="font-weight: 600; font-size: 14px">{subject}</div>'
            '<pre style="margin: 0; font-family: ui-monospace, SFMono-Regular, Menlo, monospace; font-size: 11px; line-height: 1.5; white-space: pre-wrap">'
            + "\n".join(body) + '</pre></div>')


def frame(letter, inner, caption, w):
    return (f'<div style="display: flex; flex-direction: column; gap: 10px; flex: none">{inner}'
            f'<div style="width: {w}px; color: #aeb6c4; font-size: 12px; line-height: 1.45"><span style="color: #88c0d0; font-weight: 600">{letter}</span> {caption}</div></div>')


def board(title, intro, rows, w, h, name, html_title):
    rows_html = "".join(f'<div style="display: flex; flex-direction: row; gap: 40px; align-items: flex-start">{"".join(r)}</div>' for r in rows)
    doc = f"""<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<title>{html_title}</title>
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
<div style="width: {w}px; height: {h}px; box-sizing: border-box; background: #1a1d22; padding: 30px; display: flex; flex-direction: column; gap: 24px; font-family: -apple-system, BlinkMacSystemFont, system-ui, sans-serif; font-size: 13px; line-height: 1.5; color: #eceff4">
<div style="display: flex; flex-direction: column; gap: 4px"><div style="font-size: 20px; font-weight: 600">{title}</div><div style="color: #aeb6c4; max-width: 1500px">{intro}</div></div>
{rows_html}
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
    (OUT / f"{name}.dc.html").write_text(doc)


LINE = "An idea, something that bugged you, something that broke — it goes straight to the person who makes shroom."
PH = "What’s on your mind?"

# ---------------------------------------------------------------- A
board(
    "A · Send feedback — a fourth icon in the popover, its own small window (the pick)",
    "Two doors, one form. The popover's header gains a speech-bubble icon before Library · Settings · Quit; it opens a small "
    "Send feedback window. The log goes along unless it's unticked, and “See exactly what’s sent” shows it word for word. "
    "Settings' Report a problem… stays as it is (a problem, the log always). Both post to shroom.am/report: this one as kind "
    "feedback. New words are proposals, for Bagrat.",
    [[
        frame("A1", config_popover("feedback", "Send feedback"),
              "The header: Send feedback · Library · Settings · Quit. Quit stays last. Hover names it, as the others do. "
              "Only in the popover's ready face, like its neighbours: never mid-take.", 520),
        frame("A2", window("Send feedback", form(title=None, line=LINE, placeholder=PH)),
              "Its own window, over whatever is in front, so the popover can close. Send is grey until something is typed. "
              "The log box is ticked: the default is the useful one, and it says plainly what goes and what never does.", 520),
        frame("A3", window("Send feedback", form(title=None, line=LINE, placeholder=PH, words=WORDS, mail="you@example.com", log_note=False, open_=True)),
              "Filled, with what's sent shown. Unticking the box takes the log and the setup lines out of the preview, "
              "leaving the words and the version. ⌘Return sends.", 520),
        frame("A4", window("Send feedback", sent_body()),
              "Sent: the window says so and offers Done (Return or Esc). Sending… and a failed send are Report a problem's, "
              "word for word (the mail link as the fallback).", 520),
    ], [
        frame("A5", inbox("Feedback, from the app — shroom 0.1.18", "the app (Send feedback)"),
              "What reaches reports@: the same endpoint and message, kind feedback, so the subject starts “Feedback”. "
              "“Sent from” names the door.", 520),
        frame("A6", inbox("Feedback, from the app — shroom 0.1.18", "the app (Send feedback)", log=False),
              "With the box unticked: the words and the version only.", 520),
    ]],
    2340, 1100, "FeedbackA", "shroom Send feedback, A: its own window")

# ---------------------------------------------------------------- B
SEG = ('<div style="display: flex; flex-direction: row; background: #262a31; border-radius: 7px; padding: 2px; gap: 2px; align-self: flex-start">'
       '<button style="font: inherit; border: 0; border-radius: 5px; background: {a}; color: #eceff4; padding: 3px 14px; {aw}" data-said="send-feedback.kind-feedback">Feedback</button>'
       '<button style="font: inherit; border: 0; border-radius: 5px; background: {b}; color: #eceff4; padding: 3px 14px; {bw}" data-said="send-feedback.kind-problem">A problem</button></div>')
seg_fb = SEG.format(a="#4c566a", aw="font-weight: 600", b="transparent", bw="")
seg_pr = SEG.format(a="transparent", aw="", b="#4c566a", bw="font-weight: 600")
LINK = ('<div style="display: flex; flex-direction: row; justify-content: center; gap: 4px; color: #aeb6c4; font-size: 12px; border-top: 1px solid #434a56; padding: 10px 0 0 0">'
        '<span data-said="config.something-to-tell-us">Something to tell us?</span><a href="#" style="color: #88c0d0; text-decoration: none" data-said="config.send-feedback">Send feedback</a></div>')
board(
    "B · Tell us — one door, one window, two kinds (Feedback · A problem)",
    "One door. The popover's foot gains a quiet line, “Something to tell us? Send feedback”, readable without hovering. "
    "It opens one window with a two-way switch; Settings' Report a problem… opens the same window on A problem, and its sheet "
    "goes. Feedback carries the log unless unticked; A problem always carries it, as today. New words are proposals, for Bagrat.",
    [[
        frame("B1", popover(header(("library", "settings", "quit")) + CARD + START + LINK, h=500),
              "The entry as words, under Start recording: no new icon to learn, and it reads what it does. "
              "Costs one line of the popover's height.", 520),
        frame("B2", window("Send feedback", form(title=None, kind=seg_fb, line=LINE, placeholder=PH, words=WORDS, mail="you@example.com")),
              "Feedback: the log box, ticked. The switch sets the kind the report is tagged with.", 520),
        frame("B3", window("Send feedback", form(title=None, kind=seg_pr, line="Tell us what happened. It goes straight to the person who makes shroom.",
                                                 placeholder="What were you doing, and what went wrong?", words="",
                                                 fixed_log="shroom adds its version, your Mac’s model and its recent log — never your videos, links, names or keys.")),
              "A problem: Report a problem's words and its fixed log line, no box. Settings' “Something off?” row opens it here.", 520),
        frame("B4", window("Send feedback", sent_body()),
              "Sent, as A4.", 520),
    ]],
    2340, 820, "FeedbackB", "shroom Send feedback, B: one window, two kinds")

# ---------------------------------------------------------------- C
def pop_compose(words="", sending=False):
    t = words or PH
    col = "#eceff4" if words else "#7b8494"
    send = f'<button style="{GREY}">Sending…</button>' if sending else f'<button style="{FILLED if words else GREY}">Send</button>'
    return popover(
        '<div style="display: flex; flex-direction: row; align-items: center; gap: 8px">'
        f'<div style="font-size: 15px; font-weight: 600; flex: 1 1 0" data-said="send-feedback.title">Send feedback</div></div>'
        f'<div style="color: #c3cad6; font-size: 12px" data-said="send-feedback.line-short">It goes straight to the person who makes shroom.</div>'
        f'<div style="{FIELD}; height: 96px; padding: 8px 10px; color: {col}; font-size: 12.5px">{t}</div>'
        f'<div style="{FIELD}; height: 30px; padding: 0 10px; display: flex; align-items: center; color: #7b8494; font-size: 12.5px" data-said="send-feedback.email-short">Your email, for an answer (optional)</div>'
        + check(True, "Include shroom’s recent log", "send-feedback.include-log")
        + disclosure(False)
        + f'<div style="display: flex; flex-direction: row; justify-content: space-between; gap: 8px"><button style="{BTN}">Back</button>{send}</div>',
        h=500)

board(
    "C · Send feedback — inside the popover, no window",
    "No new window. The header's speech-bubble icon (as A1) turns the popover itself into a short form, the way the quit "
    "question does. Lightest to reach and to build; the narrowest room to write in, and “See exactly what’s sent” opens "
    "the popover taller. What's typed is kept if the popover closes before Send. New words are proposals, for Bagrat.",
    [[
        frame("C1", config_popover("feedback", "Send feedback"), "The entry, as A1.", 520),
        frame("C2", pop_compose(), "The form in the popover: the words, the email, the log box and Back · Send.", 520),
        frame("C3", pop_compose(WORDS), "Filled. Return types a new line; ⌘Return sends.", 520),
        frame("C4", config_popover(extra=status_line("Sent — thank you. If you left your email, you’ll hear back.", said="settings-general.sent")),
              "Sent: the popover is back to ready, with Settings' ratified thank-you as its status line until it closes.", 520),
    ]],
    2340, 820, "FeedbackC", "shroom Send feedback, C: in the popover")
