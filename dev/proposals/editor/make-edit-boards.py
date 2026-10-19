#!/usr/bin/env python3
"""Writes the boards for Edit in shroom on the owner page (N69, B221) into <root>/project/:
PageEditButton (the page's half) and EditFromPage (what the Mac does). Approved as drawn by
Bagrat, 2026-10-18, with option A and the tooltip “Edit in shroom”.

The helpers are the editor column's (make-boards.py beside this file), so the boards read as
the column does."""

import importlib.util
import sys
from pathlib import Path

spec = importlib.util.spec_from_file_location("mb", Path(__file__).with_name("make-boards.py"))
mb = importlib.util.module_from_spec(spec)
spec.loader.exec_module(mb)

BG, CARD, LINE, FIELD, BORDER = mb.BG, mb.CARD, mb.LINE, mb.FIELD, mb.BORDER
FG, FG3, MUTED, DIM = mb.FG, mb.FG3, mb.MUTED, mb.DIM
ACCENT, RED, AMBER, MENU = mb.ACCENT, mb.RED, mb.AMBER, mb.MENU
FONT = mb.FONT

TITLE = "Where the 502s come from"
OTHER = "Onboarding walkthrough"


def page_shell(title, w, h, body, bg=BG):
    """mb.page with the page column's background."""
    return mb.page(title, w, h, body).replace("background: #1f2228; display: flex; flex-direction: column;", f"background: {bg}; display: flex; flex-direction: column;", 1)


def label(text):
    return f'<span style="color: {MUTED}; font-size: 12px; text-transform: uppercase; letter-spacing: 0.06em">{text}</span>'


def note(text, size=13):
    return f'<span style="color: {MUTED}; font-size: {size}px">{text}</span>'


def icon(path, colour=FG3, width=1.4):
    return (f'<svg viewBox="0 0 16 16" aria-hidden="true" style="width: 18px; height: 18px; fill: none; stroke: {colour}; stroke-width: {width}; '
            f'stroke-linecap: round; stroke-linejoin: round; display: block">{path}</svg>')


LINK = '<path d="M6.8 9.2a2.8 2.8 0 0 0 4 0l2.2-2.2a2.8 2.8 0 0 0-4-4l-.8.8M9.2 6.8a2.8 2.8 0 0 0-4 0L3 9a2.8 2.8 0 0 0 4 4l.8-.8"></path>'
THUMB = ('<path d="M8.2 12.8H3.3a1.5 1.5 0 0 1-1.5-1.5V4.7a1.5 1.5 0 0 1 1.5-1.5h9.4a1.5 1.5 0 0 1 1.5 1.5v3.5M7 6.2v3.6l3-1.8zM11.89 12.95a1.232 1.232 0 0 0 1.76 0l.968-.968a1.232 '
         '1.232 0 0 0-1.76-1.76l-.352.352M12.95 11.89a1.232 1.232 0 0 0-1.76 0L10.22 12.86a1.232 1.232 0 0 0 1.76 1.76l.352-.352"></path>')
DOWNLOAD = '<path d="M8 2.5v8M4.5 7.5L8 11l3.5-3.5M3 13.5h10"></path>'
# The editor's mark, as its cuts carry it on the timeline.
SCISSORS = '<circle cx="4.5" cy="4.5" r="2"></circle><circle cx="4.5" cy="11.5" r="2"></circle><path d="M6.2 5.6L13 11.8M6.2 10.4L13 4.2"></path>'


def quiet(name, path, lit=False, said=None):
    s = f' data-said="{said}"' if said else ""
    bg = "rgba(136, 192, 208, 0.12)" if lit else "transparent"
    return (f'<button aria-label="{name}"{s} style="width: 44px; height: 44px; border: 0; border-radius: 9px; background: {bg}; display: flex; align-items: center; justify-content: center; flex: none">'
            f'{icon(path)}</button>')


def head(lit=True, after=""):
    """The owner page's title row: the title field, then the quiet icons, Edit last."""
    return (f'<div style="display: flex; flex-direction: row; align-items: center; gap: 4px; padding: 8px 12px; background: {FIELD}; border-radius: 12px">'
            f'<input type="text" aria-label="Title" value="{TITLE}" style="font: inherit; font-size: 1.5rem; font-weight: 600; flex: 1 1 0; min-width: 0; box-sizing: border-box; height: 48px; padding: 0; border: 1px solid transparent; background: transparent; color: {FG}; text-overflow: ellipsis">'
            f'{quiet("Copy link", LINK)}{quiet("Copy thumbnail", THUMB)}{quiet("Download", DOWNLOAD)}{quiet("Edit in shroom", SCISSORS, lit=lit, said="owner-page.edit-in-shroom")}'
            f'</div>{after}')


def tip(text, right=6):
    return (f'<div style="align-self: flex-end; margin-right: {right}px; padding: 4px 10px; border-radius: 6px; background: {MENU}; border: 1px solid rgba(255, 255, 255, 0.12); color: {FG}; font-size: 12px">{text}</div>')


def player(h=250):
    play = '<svg viewBox="0 0 16 16" aria-hidden="true" style="width: 22px; height: 22px; fill: #eceff4; display: block"><path d="M5 3v10l8.5-5z"></path></svg>'
    return (f'<div style="height: {h}px; border-radius: 12px; background: #1f2228; display: flex; align-items: center; justify-content: center; flex: none">'
            f'<div style="width: 60px; height: 60px; border-radius: 30px; background: rgba(236, 239, 244, 0.16); display: flex; align-items: center; justify-content: center">{play}</div></div>')


def browser_question():
    """The browser's own question, drawn generically (each browser has its own look)."""
    return (f'<div style="width: 420px; box-sizing: border-box; background: #f4f5f7; border-radius: 12px; box-shadow: 0 12px 32px rgba(0, 0, 0, 0.45); padding: 18px 20px; display: flex; flex-direction: column; gap: 12px; color: #1f2328; font-size: 13px">'
            f'<span style="font-size: 15px; font-weight: 600">Open shroom?</span>'
            f'<span style="color: #4b5563">shroom.example.workers.dev wants to open this application.</span>'
            f'<label style="display: flex; flex-direction: row; gap: 8px; align-items: flex-start; color: #374151"><span style="width: 14px; height: 14px; margin-top: 2px; border-radius: 3px; border: 1px solid #9ca3af; background: #ffffff; flex: none"></span>'
            f'<span>Always allow shroom.example.workers.dev to open links of this type in the associated app</span></label>'
            f'<div style="display: flex; flex-direction: row; justify-content: flex-end; gap: 8px">'
            f'<button style="font: inherit; height: 30px; padding: 0 14px; border: 1px solid #d1d5db; border-radius: 7px; background: #ffffff; color: #1f2328">Cancel</button>'
            f'<button style="font: inherit; height: 30px; padding: 0 14px; border: 0; border-radius: 7px; background: #1a73e8; color: #ffffff; font-weight: 600">Open shroom</button></div></div>')


def mini_editor(title, inner, w=420):
    return (f'<div style="width: {w}px; box-sizing: border-box; background: {BG}; border-radius: 10px; border: 1px solid rgba(255, 255, 255, 0.1); box-shadow: 0 12px 32px rgba(0, 0, 0, 0.45); overflow: hidden; display: flex; flex-direction: column; flex: none">'
            f'{mb.chrome("Edit — " + title)}<div style="padding: 4px 16px 14px 16px; display: flex; flex-direction: column; gap: 10px">{inner}</div></div>')


def editor_getting(w, pw=300):
    return (f'<div style="display: flex; flex-direction: row; justify-content: center">{mb.preview(pw, round(pw * 9 / 16), "The 502s start at the load balancer.")}</div>'
            f'{mb.transport("0:00", "4:32")}'
            f'<div style="height: 54px; border-radius: 6px; background: {FIELD}; display: flex; flex-direction: column; align-items: center; justify-content: center; gap: 8px">'
            f'<span style="color: {MUTED}">Getting the video from your space…</span>'
            f'<div style="width: 200px; height: 4px; border-radius: 2px; background: {LINE}; overflow: hidden"><div style="width: 40%; height: 4px; background: {ACCENT}"></div></div></div>')


def change(title, text):
    return (f'<div style="display: flex; flex-direction: column; gap: 4px"><span style="font-weight: 600; font-size: 13px">{title}</span>{note(text, 12)}</div>')


def board_page():
    w, h = 1280, 1180
    left = f'''<div style="flex: 1 1 0; display: flex; flex-direction: column; gap: 14px">
{label("The owner's page — Edit, the last of the quiet icons")}
{head(after=tip("Edit in shroom", right=12))}
{player(220)}
{note("Only on the owner page: the space's owners on every video, a member on their own (the browsers that already see the title field). It shows once the video is fully up, as Edit… does in the Library, so it comes in as the upload line goes. A recipient's page and the embed never have it, so Copy link, Copy thumbnail and Download stay where they are for everyone.")}
{note("Its tooltip names it, for screen readers too: “Edit in shroom”, because the press leaves the browser. (Or “Edit…”, as the Library says it.) The scissors are the editor's own mark, the one its cuts carry.")}
<div style="height: 4px"></div>
{label("What changes")}
{change("The page", "One link in the owner page's title row: shroom://edit/&lt;your space's address&gt;/&lt;the video&gt;. It carries nothing secret, and the Mac decides for itself. There's no new address on the space, no new request and no change to the owner's cookie. The page changes, so it ships under a new page version (B61), in the release that brings the Mac's half.")}
{change("The Mac", "The app already claims shroom:// for invites (B66) and learns this second kind of link. A dev build never claims it, so the hosted Mac's drive hands the link in, as the join drive does. Any page can make such a link: it only asks, the browser asks you first, and the Mac opens only a video it could open from its own Library.")}
{change("The spec", "A new row: the page's Edit, its words, and the Mac's answers (the next board), with the quiet-take law holding a press made during a take.")}
</div>'''
    right = f'''<div style="width: 460px; display: flex; flex-direction: column; gap: 14px; flex: none">
{label("Pressed — the browser asks first, the first time")}
{browser_question()}
{note("The browser's own question, in its own words and look (Safari: “Do you want to allow this page to open “shroom”?”). With Always allow, the next press goes straight through.")}
{label("Then shroom brings its editor forward on this video")}
{mini_editor(TITLE, editor_getting(420, 340), w=420)}
{note("The editor as Edit… in the Library opens it: from the take when this Mac recorded it, or after “Getting the video from your space…”. What the Mac does when it can't is the next board.")}
</div>'''
    option_a = f'''<div style="flex: 1 1 0; display: flex; flex-direction: column; gap: 10px; background: {CARD}; border-radius: 10px; padding: 16px 18px; border: 1px solid {ACCENT}">
<span style="font-weight: 600">A — only where it can work (Bagrat's pick, 2026-10-18)</span>
{note("The owner page is only ever opened by a Mac running shroom: its one-time key is what makes a browser the owner's. So the button rides on that page and nowhere else, and needs no test. A browser can't tell whether an app takes a link, and a timer's guess would fire behind its own “Open shroom?”.")}
{note("The one miss: a Mac whose shroom is older than its space's page. The press does nothing; updating shroom fixes it.")}
</div>'''
    option_b = f'''<div style="flex: 1 1 0; display: flex; flex-direction: column; gap: 10px; background: {CARD}; border-radius: 10px; padding: 16px 18px">
<span style="font-weight: 600">B — the same, plus a quiet line, the invite page's way (not chosen)</span>
{note("Pressed, one line shows under the title row for a few seconds. When shroom opens, its editor covers the page and the line goes as the page loses focus, so it's only seen when nothing opened.")}
<div style="display: flex; flex-direction: column; gap: 6px; transform: scale(0.82); transform-origin: left top; width: 122%">
{head(lit=False)}
<span style="color: {MUTED}; font-size: 13px; padding-left: 12px">If shroom didn't open, it may need an update.</span></div>
</div>'''
    body = f'''<div style="flex: 1 1 0; padding: 36px 40px; display: flex; flex-direction: column; gap: 28px">
<div style="display: flex; flex-direction: row; gap: 40px">{left}{right}</div>
<div style="display: flex; flex-direction: column; gap: 12px">{label("Where shroom isn't on the Mac — browsers show nothing for a link no app takes")}
<div style="display: flex; flex-direction: row; gap: 20px">{option_a}{option_b}</div></div>
</div>'''
    return page_shell("shroom video page, Edit in shroom", w, h, body)


def mini_popover(line, said=None):
    """The ready-to-record popover under the mushroom, its one-shot status line at the foot."""
    s = f' data-said="{said}"' if said else ""
    logo = ('<svg viewBox="18 20 84 90" aria-hidden="true" style="width: 17px; height: 18px; display: block">'
            '<path d="M50,68 C49,82 46,92 46,96 C46,101 52,104 60,104 C68,104 74,101 74,96 C74,92 71,82 70,68 Z" fill="#E5E9F0"></path>'
            '<path d="M24,64 C24,42 40,26 60,26 C80,26 96,42 96,64 C86,75 34,75 24,64 Z" fill="#88C0D0"></path>'
            '<circle cx="33" cy="58" r="3.4" fill="#ECEFF4"></circle><circle cx="88" cy="58" r="2.2" fill="#ECEFF4"></circle><circle cx="50" cy="44" r="6" fill="#ECEFF4"></circle>'
            '<circle cx="72" cy="48" r="3.6" fill="#ECEFF4"></circle><circle cx="64" cy="58" r="2.6" fill="#ECEFF4"></circle></svg>')
    caret = f'<svg viewBox="0 0 12 12" aria-hidden="true" style="width: 10px; height: 10px; fill: none; stroke: {MUTED}; stroke-width: 1.5; stroke-linecap: round; stroke-linejoin: round; display: block; flex: none"><path d="M3 4.5l3 3 3-3"></path></svg>'

    def src(path, text, top=True):
        return (f'<div style="height: 34px; padding: 0 10px; {"border-top: 1px solid " + LINE + ";" if top else ""} display: flex; flex-direction: row; align-items: center; gap: 10px">'
                f'<svg viewBox="0 0 16 16" aria-hidden="true" style="width: 16px; height: 16px; fill: none; stroke: {MUTED}; stroke-width: 1.4; stroke-linecap: round; stroke-linejoin: round; display: block; flex: none">{path}</svg>'
                f'<span style="flex: 1 1 0; white-space: nowrap; overflow: hidden; text-overflow: ellipsis">{text}</span>{caret}</div>')
    rows = (src('<rect x="1.5" y="2.5" width="13" height="9" rx="1.5"></rect><path d="M6 14h4M8 11.5V14"></path>', "Built-in Display", top=False)
            + src('<rect x="6" y="1.5" width="4" height="8" rx="2"></rect><path d="M3.5 7.5a4.5 4.5 0 0 0 9 0M8 12v2.5"></path>', "MacBook Pro Microphone")
            + src('<rect x="1.5" y="4" width="9" height="8" rx="1.8"></rect><path d="M10.5 7l4-2v6l-4-2"></path>', "FaceTime HD Camera"))
    return (f'<div style="width: 300px; box-sizing: border-box; background: {BG}; border: 1px solid rgba(255, 255, 255, 0.1); border-radius: 12px; box-shadow: 0 8px 28px rgba(0, 0, 0, 0.3); padding: 12px 14px 14px 14px; display: flex; flex-direction: column; gap: 12px; flex: none">'
            f'<div style="display: flex; flex-direction: row; align-items: center; gap: 7px">{logo}<span style="font-size: 14px; font-weight: 600">shroom</span></div>'
            f'<div style="background: {CARD}; border-radius: 9px; display: flex; flex-direction: column">{rows}</div>'
            f'<button style="font: inherit; font-weight: 600; height: 36px; border: 0; border-radius: 8px; background: {ACCENT}; color: {BG}">Start recording</button>'
            f'<div style="display: flex; flex-direction: row; align-items: flex-start; gap: 7px; color: {MUTED}; font-size: 12px">'
            f'<span style="margin-top: 5px; width: 6px; height: 6px; border-radius: 3px; background: {DIM}; display: block; flex: none"></span><div{s}>{line}</div></div></div>')


def board_mac():
    w, h = 1100, 1320
    opens = f'''<div style="display: flex; flex-direction: row; gap: 24px; align-items: flex-start">
{mini_editor(TITLE, editor_getting(380), w=380)}
{note("The same window Edit… in the Library opens: one at a time, playing from the take when this Mac recorded the video, else fetching it from your space first. If the video isn't on this Mac and your space can't be reached, it says so in its own words: “Couldn't get the video. Check your connection, then try again.”, with Try again.")}
</div>'''
    busy_inner = (f'<div style="height: 54px; border-radius: 6px; background: {FIELD}; display: flex; align-items: center; justify-content: center; color: {DIM}">the timeline, as it was left</div>'
                  + mb.footer('<span data-said="editor.editing-another">You\'re editing another video — press Done, then Edit again.</span>', line_colour=AMBER))
    busy = f'''<div style="display: flex; flex-direction: row; gap: 24px; align-items: flex-start">
{mini_editor(OTHER, busy_inner, w=640)}
{note("It doesn't swap videos under unsaved changes. The line takes the footer's place until the next change. While it makes a final video or asks about the original, it comes forward as it is: those faces already say what to press. Pressed for the video it already holds, it just comes forward.")}
</div>'''
    take = f'''<div style="display: flex; flex-direction: row; gap: 24px; align-items: center">
<div style="width: 300px; flex: none; display: flex; flex-direction: column; gap: 8px; padding: 12px 14px; border-radius: 12px; background: {BG}; border: 1px solid rgba(255, 255, 255, 0.1)">
<span style="font-size: 15px; font-weight: 600">Paused — 4:32</span><span style="color: {DIM}">nothing new on screen</span></div>
{note("While a take is open, recording or paused, shroom puts nothing new on screen (law 4, the quiet-take law), as with an invite that arrives mid-take. The press waits. Once the take ends, by Finish &amp; Share or Discard…, the editor comes forward on the video it asked for. A second press while waiting replaces the first.")}
</div>'''
    cant = f'''<div style="display: flex; flex-direction: row; gap: 26px; align-items: flex-start">
<div style="display: flex; flex-direction: column; gap: 8px">{note("This Mac is set up for another space", 12)}{mini_popover("This Mac isn't set up for that video's space.", said="config.this-mac-isnt-set-up")}</div>
<div style="display: flex; flex-direction: column; gap: 8px">{note("A member's Mac, and someone else's video", 12)}{mini_popover("Only whoever recorded this video, or the space's owner, can edit it.", said="config.only-whoever-recorded-this-video")}</div>
<div style="display: flex; flex-direction: column; gap: 8px">{note("Deleted since the page opened", 12)}{mini_popover("That video isn't in your space anymore.", said="config.that-video-isnt-in-your")}</div>
</div>
{note("The popover opens by itself under the mushroom, as it does after an update: the app you were in keeps the keyboard, and a click elsewhere closes it. The line is said once, until the popover closes (a status line, law 7: facts, not asks). A Mac not set up at all opens its welcome window instead, as a click on the mushroom does.")}'''
    body = f'''<div style="flex: 1 1 0; padding: 24px; display: flex; flex-direction: column; gap: 20px">
{mb.strip("The video is one this Mac can edit (the owner's Macs: every video in the space; a member's Mac: their own): the editor opens on it", opens)}
{mb.strip("The editor is open on another video: it comes forward as it is, one at a time, and says why", busy)}
{mb.strip("A take is open: the press waits until the take ends", take)}
{mb.strip("This Mac can't edit that video: one line, and nothing else changes", cant)}
{note("The link only asks. Before acting, the Mac checks its own state: its space, what it may edit (as its Library shows it, asking your space when the video is newer than the list), and whether a take is open. Nothing is saved or changed by the press itself.", 12)}
</div>'''
    return mb.page("shroom Editor, opened from the page", w, h, body)


def main():
    root = Path(sys.argv[1]) / "project"
    root.mkdir(parents=True, exist_ok=True)
    (root / "PageEditButton.dc.html").write_text(board_page())
    (root / "EditFromPage.dc.html").write_text(board_mac())


if __name__ == "__main__":
    main()
