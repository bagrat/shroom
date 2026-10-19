#!/usr/bin/env python3
"""Writes the board for Edit pressed on another video while the editor is open into
<root>/project/: EditorSwitching. Bagrat found, 2026-10-19, that the editor stays on the
video it holds when Edit is pressed for another one, and that the line saying so isn't
enough: with nothing to save it should switch straight to the other video; with changes
not saved, the line stays (N69) until the close prompt (EditorClosing) is built. A
proposal, for him to see before anything is built — superseded the same day by one
window per video (EditorWindows, make-windows-board.py; N73), which he chose instead.
The helpers are the editor column's (make-boards.py, make-edit-boards.py and
make-close-board.py beside this file), so the board reads as the column does."""

import importlib.util
import sys
from pathlib import Path


def load(name, file):
    spec = importlib.util.spec_from_file_location(name, Path(__file__).with_name(file))
    m = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(m)
    return m


mb = load("mb", "make-boards.py")
eb = load("eb", "make-edit-boards.py")
cb = load("cb", "make-close-board.py")
FIELD, LINE, ACCENT, AMBER = mb.FIELD, mb.LINE, mb.ACCENT, mb.AMBER
FG, FG3, MUTED, DIM = mb.FG, mb.FG3, mb.MUTED, mb.DIM

FOOTER = "Your link stays the same. Viewers see the edit once it's saved."
STRIP_W = 1152


def editor_on(title, caption, length, seconds, seed, w=450, pw=290):
    """The editor on a video with nothing changed, paused at its start."""
    inner = (f'<div style="display: flex; flex-direction: row; justify-content: center">{mb.preview(pw, round(pw * 9 / 16), caption)}</div>'
             f'{mb.transport("0:00", length)}'
             f'<div style="padding-top: 8px">{mb.timeline(w - 32, seconds, playhead=0, seed=seed)}</div>'
             f'{mb.footer(FOOTER, revert=False)}')
    return eb.mini_editor(title, inner, w=w)


def pressed(text):
    """What was pressed, over an arrow to the window it led to."""
    arrow = (f'<svg viewBox="0 0 120 16" aria-hidden="true" style="width: 120px; height: 16px; display: block; fill: none; stroke: {ACCENT}; stroke-width: 1.6; stroke-linecap: round; stroke-linejoin: round">'
             '<path d="M2 8h112M106 2l8 6-8 6"></path></svg>')
    return (f'<div style="width: 150px; flex: none; align-self: center; display: flex; flex-direction: column; align-items: center; gap: 10px; text-align: center">'
            f'<span style="color: {FG3}; font-size: 12px">{text}</span>{arrow}</div>')


def making(w=450):
    """After Done: the final video being made (EditorAfterSave)."""
    bar = f'<div style="width: 200px; height: 4px; border-radius: 2px; background: {LINE}; overflow: hidden"><div style="width: 40%; height: 4px; background: {ACCENT}"></div></div>'
    inner = (f'<div style="height: 58px; border-radius: 6px; background: {FIELD}; display: flex; flex-direction: column; align-items: center; justify-content: center; gap: 8px">'
             f'<span style="color: {FG}" data-said="editor-making.making">Making the final video… 40%</span>{bar}</div>'
             f'{mb.footer("The link already plays your edit. You can close this window: shroom finishes it.", done="Close", revert=False)}')
    return eb.mini_editor(eb.TITLE, inner, w=w)


def notes(*lines, w=None):
    width = f"width: {w}px; flex: none;" if w else "flex: 1 1 0;"
    return f'<div style="{width} display: flex; flex-direction: column; gap: 10px">' + "".join(eb.note(l) for l in lines) + "</div>"


def board():
    w, h = 1200, 1640
    switch = f'''<div style="display: flex; flex-direction: row; gap: 20px; align-items: flex-start">
{editor_on(eb.TITLE, "The 502s start at the load balancer.", "4:32", 272, seed=3)}
{pressed("Edit on “" + eb.OTHER + "”, from its page or the Library")}
{editor_on(eb.OTHER, "This is where your videos live.", "2:48", 168, seed=11)}
</div>
<div style="display: flex; flex-direction: row; gap: 24px">
{notes("Nothing to save: it just opened, it's still getting the video or couldn't, every change was undone, or Done saved it. So nothing is lost: it's ✕ then Edit, in one press.")}
{notes("The same window, where it was, takes the other video: its name in the title bar, its picture, its timeline. The first one stops, and a video still coming stops, as after ✕. The other opens paused at its start, from the take when this Mac recorded it, else after “Getting the video from your space…”.")}
{notes("Either way in: Edit in shroom on the owner page, or Edit… in the Library. No new words: the switch says nothing, the title bar and the picture show it.")}
</div>'''
    after_done = f'''<div style="display: flex; flex-direction: row; gap: 24px; align-items: flex-start">
{making()}
{notes("Done saved the edit, so nothing is lost here either: the final video is still made and put up, as after ✕ (B197 (a)), and the original is kept for 2 days, as ✕ keeps it (B197 (b)).",
       "Or should it come forward as it is there, so “Delete the original?” is still asked? Bagrat's call.")}
</div>'''
    changed_line = mb.footer('<span data-said="editor.editing-another">You\'re editing another video — press Done, then Edit again.</span>', line_colour=AMBER)
    changed = f'''<div style="display: flex; flex-direction: row; gap: 24px; align-items: flex-start">
{cb.editor(changed_line)}
{notes("A press in another window never drops changes. The line takes the footer's place until the next change; Done saves, then Edit again opens the other video.",
       "It stays so until the close prompt is built (the board “Editor — closing with changes not saved”). Then this press can ask that question too: Discard opens the other video, Save opens it once the edit is saved, Keep editing stays.")}
</div>'''
    saving = mb.footer("Saving your edit…", done="Saving…", done_enabled=False)
    failed = mb.footer("Couldn't save your edit. Check your connection, then press Done again.", line_colour=AMBER)
    busy = f'''<div style="display: flex; flex-direction: row; gap: 24px; align-items: center">
<div style="flex: 1 1 0; display: flex; flex-direction: column; gap: 12px">{saving}{failed}</div>
{notes("Those faces say what's happening and what to press. Once the edit is saved, Edit switches straight to the other video.", w=380)}
</div>'''
    body = f'''<div style="flex: 1 1 0; padding: 24px; display: flex; flex-direction: column; gap: 20px">
{mb.strip("Nothing to save: Edit on another video switches straight to it, in the same window", switch, w=STRIP_W)}
{mb.strip("After Done, while the final video is made or it asks “Delete the original?”: the same, it switches", after_done, w=STRIP_W)}
{mb.strip("Changes not saved: it comes forward as it is, and says why (as built)", changed, w=STRIP_W)}
{mb.strip("While it saves, or when a save didn't go through or the space refused it: it comes forward as it is (as now)", busy, w=STRIP_W)}
{eb.note("Pressed for the video it already holds, it just comes forward, as now. With a take open, the press waits until the take ends, as now (the board “Editor — opened from the page's Edit”).", 12)}
{eb.note("Today it never switches: it comes forward as it is, and says why while nothing is saving (N69, B221 (c)). This changes that where nothing is left to save. A proposal, for Bagrat.", 12)}
</div>'''
    return mb.page("shroom Editor, Edit on another video", w, h, body)


def main():
    root = Path(sys.argv[1]) / "project"
    root.mkdir(parents=True, exist_ok=True)
    (root / "EditorSwitching.dc.html").write_text(board())


if __name__ == "__main__":
    main()
