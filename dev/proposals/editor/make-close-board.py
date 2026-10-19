#!/usr/bin/env python3
"""Writes the board for closing the editor with changes not saved into <root>/project/:
EditorClosing. Bagrat asked, 2026-10-18, that ✕ stop saving without a word, as Done does
(B195): it should ask — discard, keep editing, or save. A proposal, for him to see before
anything is built; the words are placeholders.
The helpers are the editor column's (make-boards.py and make-edit-boards.py beside this
file), so the board reads as the column does."""

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
CARD, LINE = mb.CARD, mb.LINE
FG3, MUTED, RED = mb.FG3, mb.MUTED, mb.RED


def ask(title, line, buttons):
    """The window's question card, drawn as “Delete the original?” is (B197)."""
    return (f'<div style="display: flex; flex-direction: row; align-items: center; gap: 12px; background: {CARD}; border: 1px solid {LINE}; border-radius: 10px; padding: 14px 16px">'
            f'<div style="flex: 1 1 0; display: flex; flex-direction: column; gap: 4px">'
            f'<span style="font-weight: 600; font-size: 14px">{title}</span><span style="color: {MUTED}">{line}</span></div>{buttons}</div>')


def editor(bottom, w=700):
    """The editor on a video with a cut, `bottom` in its footer's place."""
    pw = 300
    inner = (f'<div style="display: flex; flex-direction: row; justify-content: center">{mb.preview(pw, round(pw * 9 / 16), "The 502s start at the load balancer.")}</div>'
             f'{mb.transport("1:16", "3:39", "4:32")}'
             f'<div style="padding-top: 26px">{mb.timeline(w - 32, 272, cuts=((150, 162),), trim=(8, 241), playhead=84, seed=3)}</div>'
             f'{bottom}')
    return eb.mini_editor(eb.TITLE, inner, w=w)


def board():
    w, h = 1100, 920
    asking = ask("Save your edit?", "What you've changed isn't saved yet.",
                 mb.button("Discard", colour=RED) + mb.button("Keep editing", colour=FG3) + mb.button("Save", primary=True))
    closing = f'''<div style="display: flex; flex-direction: row; gap: 24px; align-items: flex-start">
{editor(asking)}
<div style="display: flex; flex-direction: column; gap: 12px">
{eb.note("Save is Done: the edit is saved, then the final video is made, as after Done. Return presses it.")}
{eb.note("Keep editing puts the question away, every change still there. Esc does the same.")}
{eb.note("Discard closes the window and drops what wasn't saved: the video stays as it was last saved. It's never on Return.")}
</div></div>'''
    body = f'''<div style="flex: 1 1 0; padding: 24px; display: flex; flex-direction: column; gap: 20px">
{mb.strip("✕ with changes not saved: the window asks, in its footer's place, the edit held behind it", closing)}
{mb.strip("Nothing to save", eb.note("Nothing changed since it opened, or since the last save: ✕ closes the window at once, as it does now."))}
{mb.strip("After Done", eb.note("Saving, making the final video, or asking about the original: ✕ is Close, as now. Nothing is lost there: the edit is already saved."))}
{mb.strip("Quit and Update", eb.note("With changes not saved, they save first, as now, and shroom quits once the edit is saved (B199). Should they ask the same three ways? Bagrat's call."))}
{eb.note("Today ✕ is Done: it saves without asking (B195). This changes that for ✕ alone. A proposal, for Bagrat: the words are placeholders.", 12)}
</div>'''
    return mb.page("shroom Editor, closing with changes not saved", w, h, body)


def main():
    root = Path(sys.argv[1]) / "project"
    root.mkdir(parents=True, exist_ok=True)
    (root / "EditorClosing.dc.html").write_text(board())


if __name__ == "__main__":
    main()
