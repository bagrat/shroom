#!/usr/bin/env python3
"""Rewrites two editor boards into <root>/project/: EditorClosing and EditorAfterSave.

Bagrat, hand-testing 0.1.30-early.1 (2026-10-25): the editor's two questions sat in the
footer's place and were too easy to miss. ✕'s “Save your edit?” (N71) and “Delete the
original?” (B197) become sheets over the window, drawn as the Library's Delete sheet is
(app/src/library/sheet.css): the window dimmed, one card near its top, its title, its
line, and its buttons right-aligned in the Scene's order. The words are as they are.
The helpers are the editor column's (../editor/make-boards.py, make-edit-boards.py)."""

import importlib.util
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent


def load(name, file):
    spec = importlib.util.spec_from_file_location(name, HERE.parent / "editor" / file)
    m = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(m)
    return m


mb = load("mb", "make-boards.py")
eb = load("eb", "make-edit-boards.py")
BG, CARD, BORDER, LINE = mb.BG, mb.CARD, mb.BORDER, mb.LINE
FG, FG3, MUTED, DIM, ACCENT, RED, AMBER = mb.FG, mb.FG3, mb.MUTED, mb.DIM, mb.ACCENT, mb.RED, mb.AMBER
FG2 = "#d8dee9"
SCRIM = "rgba(46, 52, 64, 0.62)"


def sheet_button(label, kind, said=None):
    """A sheet's button (sheet.css): 32 high, radius 8, plain, red-outlined or filled."""
    s = f' data-said="{said}"' if said else ""
    base = "font: inherit; height: 32px; min-width: 82px; padding: 0 14px; border-radius: 8px; flex: none; white-space: nowrap"
    look = {
        "plain": f"border: 1px solid {BORDER}; background: transparent; color: {FG}",
        "red": f"border: 1px solid {RED}; background: transparent; color: {RED}",
        "filled": f"border: 1px solid {ACCENT}; background: {ACCENT}; color: {BG}; font-weight: 600",
    }[kind]
    return f'<button style="{base}; {look}"{s}>{label}</button>'


def sheet(title, line, buttons, said=None):
    t = f' data-said="{said}.title"' if said else ""
    l = f' data-said="{said}.line"' if said else ""
    return (f'<div role="alertdialog" style="width: 440px; box-sizing: border-box; background: {CARD}; border: 1px solid rgba(255, 255, 255, 0.1); border-radius: 12px; '
            f'box-shadow: 0 8px 28px rgba(0, 0, 0, 0.3); padding: 20px; display: flex; flex-direction: column; gap: 8px">'
            f'<span style="font-size: 15px; font-weight: 600; line-height: 1.4; margin-bottom: -2px"{t}>{title}</span>'
            f'<span style="color: {FG2}; line-height: 1.5"{l}>{line}</span>'
            f'<div style="display: flex; flex-direction: row; justify-content: flex-end; gap: 8px; margin-top: 8px">{buttons}</div></div>')


def footer(line, *buttons):
    return (f'<div style="display: flex; flex-direction: row; align-items: center; gap: 10px">'
            f'<span style="color: {MUTED}; flex: 1 1 0">{line}</span>{"".join(buttons)}</div>')


LINK = "Your link stays the same. Viewers see the edit once it's saved."


def editor(bottom, over=None, w=700):
    """The editor on a video with a cut, `bottom` as its footer and, when given, a sheet
    over it all: the window under the title bar dimmed, the card near its top."""
    pw = 300
    inner = (f'<div style="display: flex; flex-direction: row; justify-content: center">{mb.preview(pw, round(pw * 9 / 16), "The 502s start at the load balancer.")}</div>'
             f'{mb.transport("1:16", "3:39", "4:32")}'
             f'<div style="padding-top: 26px">{mb.timeline(w - 32, 272, cuts=((150, 162),), trim=(8, 241), playhead=84, seed=3)}</div>'
             f'<div style="height: 6px"></div>{bottom}')
    cover = (f'<div style="position: absolute; inset: 0; background: {SCRIM}; display: flex; flex-direction: column; align-items: center; padding-top: 44px">{over}</div>'
             if over else "")
    return (f'<div style="width: {w}px; box-sizing: border-box; background: {BG}; border-radius: 10px; border: 1px solid rgba(255, 255, 255, 0.1); box-shadow: 0 12px 32px rgba(0, 0, 0, 0.45); overflow: hidden; display: flex; flex-direction: column; flex: none">'
            f'{mb.chrome("Edit — " + eb.TITLE)}'
            f'<div style="position: relative"><div style="padding: 4px 16px 16px 16px; display: flex; flex-direction: column; gap: 10px">{inner}</div>{cover}</div></div>')


def side(*notes):
    return f'<div style="display: flex; flex-direction: column; gap: 12px">{"".join(eb.note(n) for n in notes)}</div>'


def pair(win, notes):
    return f'<div style="display: flex; flex-direction: row; gap: 24px; align-items: flex-start">{win}{notes}</div>'


def disabled(label):
    return (f'<button disabled style="font: inherit; font-weight: 600; height: 30px; padding: 0 16px; border: 0; border-radius: 7px; '
            f'background: {ACCENT}; color: {BG}; opacity: 0.45; flex: none">{label}</button>')


def board_closing():
    w, h = 1100, 900
    ask = sheet("Save your edit?", "What you've changed isn't saved yet.",
                sheet_button("Discard", "red") + sheet_button("Keep editing", "plain") + sheet_button("Save", "filled"))
    behind = footer(LINK, mb.button("Revert to original", colour=FG3), mb.button("Done", primary=True))
    closing = pair(editor(behind, ask), side(
        "A sheet over the window, as the Library's Delete is: the window dimmed, one card near its top. Nothing behind it answers a click, and Space doesn't play.",
        "Save is Done: the edit is saved, then the final video is made, as after Done. Return presses it.",
        "Keep editing puts the sheet away, every change still there. Esc does the same.",
        "Discard closes the window and drops what wasn't saved: the video stays as it was last saved. It's red, and never on Return.",
    ))
    body = f'''<div style="flex: 1 1 0; padding: 24px; display: flex; flex-direction: column; gap: 20px">
{mb.strip("✕ with changes not saved: the window asks in a sheet over it, the edit held behind it", closing)}
{mb.strip("Nothing to save", eb.note("Nothing changed since it opened, or since the last save: ✕ closes the window at once."))}
{mb.strip("After Done", eb.note("Saving, making the final video, or asking about the original: ✕ is Close. Nothing is lost there: the edit is already saved."))}
{mb.strip("Quit and Update", eb.note("With changes not saved, they save first, and shroom quits once the edit is saved (B199). No sheet."))}
{eb.note("Was: the question sat in the footer's place, and was easy to miss (Bagrat, 2026-10-25). The words are as they were.", 12)}
</div>'''
    return mb.page("shroom Editor, closing with changes not saved", w, h, body)


def board_after_save():
    """After Done: the final video is made, then the original's fate is asked (B197) — in a
    sheet now; the other strips are as they were."""
    bar = lambda pct: f'<div style="width: 240px; height: 4px; border-radius: 2px; background: {LINE}; overflow: hidden"><div style="width: {pct}%; height: 4px; background: {ACCENT}"></div></div>'
    making = f'''<div style="height: 66px; border-radius: 6px; background: {mb.FIELD}; display: flex; flex-direction: column; align-items: center; justify-content: center; gap: 8px">
<span style="color: {FG}" data-said="editor-making.making">Making the final video… 40%</span>{bar(40)}</div>
{mb.footer("The link already plays your edit. You can close this window: shroom finishes it.", done="Close", revert=False)}'''
    ask = sheet("Delete the original?", "Your edit is the video now. The original is only kept so you can revert to it.",
                sheet_button("Keep for 2 days", "plain", said="editor-keep-original.keep") + sheet_button("Yes, delete", "filled", said="editor-keep-original.delete"),
                said="editor-keep-original")
    asking = pair(editor(footer(LINK, disabled("Close")), ask), side(
        "A sheet over the window, as ✕'s Save your edit? is: the window dimmed, one card near its top.",
        "Yes, delete leads, filled, and is a click only: Return does nothing here.",
        "Keep for 2 days closes the window and keeps the original two days. ✕ and Esc do the same.",
    ))
    failed = mb.footer("Couldn't make the final video. The link plays your edit anyway, and shroom tries again later.", done="Close", revert=False, line_colour=AMBER)
    kept = (f'<div style="display: flex; flex-direction: row; align-items: center; gap: 10px">'
            f'<span style="color: {MUTED}; flex: 1 1 0">{LINK}</span>'
            f'<span style="color: {DIM}; font-size: 12px" data-said="editor.original-kept">The original is kept one more day</span>'
            f'{mb.button("Revert to original", colour=FG3)}{mb.button("Done", primary=True)}</div>')
    gone = (f'<div style="display: flex; flex-direction: row; align-items: center; gap: 10px">'
            f'<span style="color: {MUTED}; flex: 1 1 0">{LINK}</span>'
            f'<button disabled style="font: inherit; height: 30px; padding: 0 14px; border: 1px solid {BORDER}; border-radius: 7px; background: transparent; color: {DIM}; flex: none; opacity: 0.6">Revert to original</button>{mb.button("Done", primary=True)}</div>')
    body = f'''<div style="flex: 1 1 0; padding: 24px; display: flex; flex-direction: column; gap: 20px">
{mb.strip("Saved: the final video is made on this Mac and put up (the link already plays the edit)", making)}
{mb.strip("Once it's up: the window asks in a sheet over it, Yes, delete leading (a click: Return does nothing here). ✕, Esc or Keep for 2 days keeps it", asking)}
{mb.strip("When it couldn't be made (offline, the space didn't answer)", failed)}
{mb.strip("Editing again while the original is kept: the time it has left sits by Revert", kept)}
{mb.strip("Once the original is gone the edit is final: Revert to original only undoes what's changed since the window opened", gone)}
</div>'''
    return mb.page("shroom Editor, after saving", 1100, 1080, body)


def main():
    root = Path(sys.argv[1]) / "project"
    root.mkdir(parents=True, exist_ok=True)
    (root / "EditorClosing.dc.html").write_text(board_closing())
    (root / "EditorAfterSave.dc.html").write_text(board_after_save())


if __name__ == "__main__":
    main()
