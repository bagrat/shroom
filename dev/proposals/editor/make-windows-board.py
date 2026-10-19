#!/usr/bin/env python3
"""Writes the board for one editor window per video into <root>/project/: EditorWindows.
Bagrat asked, 2026-10-19, whether each video could have its own editor window, the Mac's
document way, instead of one window that switches or says it's busy: Edit on a video not
open opens a window of its own; on one already open, its window comes forward. He chose it
as drawn the same day, and it was built so (N73, B227); it adds no words.
The helpers are the editor column's (make-boards.py, make-edit-boards.py,
make-close-board.py and make-switch-board.py beside this file)."""

import importlib.util
import sys
from pathlib import Path

sys.dont_write_bytecode = True


def load(name, file):
    spec = importlib.util.spec_from_file_location(name, Path(__file__).with_name(file))
    m = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(m)
    return m


mb = load("mb", "make-boards.py")
eb = load("eb", "make-edit-boards.py")
cb = load("cb", "make-close-board.py")
sb = load("sb", "make-switch-board.py")
ACCENT, RED, FG3 = mb.ACCENT, mb.RED, mb.FG3

STRIP_W = 1152
WIN_W = 500


def edited(w=WIN_W, bottom=None):
    """“Where the 502s come from” with a cut, its changes not saved yet."""
    return cb.editor(bottom or mb.footer(sb.FOOTER), w=w)


def fresh(w=WIN_W):
    """“Onboarding walkthrough”, just opened."""
    return sb.editor_on(eb.OTHER, "This is where your videos live.", "2:48", 168, seed=11, w=w, pw=280)


def making(title, w=440):
    """After Done: the final video being made (EditorAfterSave), in `title`'s window."""
    bar = f'<div style="width: 200px; height: 4px; border-radius: 2px; background: {mb.LINE}; overflow: hidden"><div style="width: 40%; height: 4px; background: {ACCENT}"></div></div>'
    inner = (f'<div style="height: 58px; border-radius: 6px; background: {mb.FIELD}; display: flex; flex-direction: column; align-items: center; justify-content: center; gap: 8px">'
             f'<span style="color: {mb.FG}" data-said="editor-making.making">Making the final video… 40%</span>{bar}</div>'
             f'{mb.footer("The link already plays your edit. You can close this window: shroom finishes it.", done="Close", revert=False)}')
    return eb.mini_editor(title, inner, w=w)


def stacked(back, front, h, dx=150, dy=56):
    """Two windows as macOS stacks them: the newer one in front, down and to the right."""
    return (f'<div style="position: relative; height: {h}px; flex: none; width: {WIN_W + dx}px">'
            f'<div style="position: absolute; left: 0; top: 0; opacity: 0.55">{back}</div>'
            f'<div style="position: absolute; left: {dx}px; top: {dy}px">{front}</div></div>')


def notes(*lines, w=None):
    width = f"width: {w}px; flex: none;" if w else "flex: 1 1 0;"
    return f'<div style="{width} display: flex; flex-direction: column; gap: 10px">' + "".join(eb.note(l) for l in lines) + "</div>"


def board():
    w, h = 1200, 1880
    opens = f'''<div style="display: flex; flex-direction: row; gap: 24px; align-items: flex-start">
{stacked(edited(), fresh(), 470)}
{notes("Edit on “" + eb.OTHER + "”, from its page or the Library, while “" + eb.TITLE + "” is open: a window of its own opens for it, in front. The first stays as it was, its changes still there, not saved and not lost.",
       "It opens as a first window does: from the take when this Mac recorded the video, else after “Getting the video from your space…”, paused at its start.")}
</div>'''
    forward = f'''<div style="display: flex; flex-direction: row; gap: 24px; align-items: flex-start">
{stacked(fresh(), edited(), 506)}
{notes("Edit on a video whose window is open, from anywhere: that window comes forward, as it was. Nothing else moves.",
       "So there's never a window to switch, and nothing to explain: the line “You're editing another video — press Done, then Edit again.” goes, and the question of switching after Done with it.")}
</div>'''
    own = f'''<div style="display: flex; flex-direction: row; gap: 24px; align-items: flex-start">
{making(eb.OTHER)}
{cb.editor(cb.ask("Save your edit?", "What you've changed isn't saved yet.", mb.button("Discard", colour=RED) + mb.button("Keep editing", colour=FG3) + mb.button("Save", primary=True)), w=640)}
</div>
<div style="display: flex; flex-direction: row; gap: 24px">
{notes("Each window is its own: its Undo, its Done, its final video and its “Delete the original?”. Done in one never touches another.")}
{notes("✕ asks in its own window, with changes not saved there (the close prompt, as approved). Every other window stays as it is.")}
{notes("Final videos are made one at a time, as now: a second Done waits its turn, its window reading “Making the final video… 0%” until then.")}
</div>'''
    body = f'''<div style="flex: 1 1 0; padding: 24px; display: flex; flex-direction: column; gap: 20px">
{mb.strip("Edit on a video that isn't open: a window of its own opens for it", opens, w=STRIP_W)}
{mb.strip("Edit on a video that's open: its window comes forward", forward, w=STRIP_W)}
{mb.strip("Each window on its own: one making its final video, another asking before it closes", own, w=STRIP_W)}
{eb.note("Quit and Update: every window's changes are saved first, as now (Bagrat, 2026-10-19). Deleting a video closes its window, and no other. With a take open, nothing new opens until it ends (law 4): an Edit pressed meanwhile waits, as now.", 12)}
{eb.note("Built as drawn (N73, B227; Bagrat chose it, 2026-10-19): one window per video, the Mac's document way, where the editor was one window, one video at a time (B195 (a)). No new words: one line went.", 12)}
</div>'''
    return mb.page("shroom Editor, one window per video", w, h, body)


def main():
    root = Path(sys.argv[1]) / "project"
    root.mkdir(parents=True, exist_ok=True)
    (root / "EditorWindows.dc.html").write_text(board())


if __name__ == "__main__":
    main()
