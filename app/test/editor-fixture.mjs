// The editor's faces, written by hand as the core will hand them over (the build contract's
// EditorBody, N63) until crates/shroom-core/examples/gallery.rs prints its `editor` rows:
// then the core's slices replace these, entry for entry (`{ window, variant, scene }`, as
// the gallery prints them). The words are the spec's said-lines; the lengths are the test
// video's (editor-preview.mjs makes it), so the slices and the media agree.

export const REC = "k3v9q2mz7x";

const labels = { play: "Play", pause: "Pause", undo: "Undo", redo: "Redo", cut: "Cut {} s", put_back: "Put back {} s", was: "was {}" };
const button = (label, action, style, enabled = true) => ({ action, label, style, enabled, note: null });
const FOOTER = "Your link stays the same. Viewers see the edit once it's saved.";
const DONE = { do: "editor-done" };
const CLOSE = { do: "editor-close" };

/** What a list plays (the core's `keep`): the trim minus the applied cuts. */
function keepOf(list) {
  const out = [];
  let at = list.in;
  const cuts = list.cuts
    .filter((c) => c.applied)
    .map((c) => [Math.max(c.start, list.in), Math.min(c.end, list.out)])
    .filter(([s, e]) => e > s)
    .sort((a, b) => a[0] - b[0]);
  for (const [s, e] of cuts) {
    if (e <= at) continue;
    if (s > at) out.push([at, s]);
    at = Math.max(at, e);
  }
  if (at < list.out) out.push([at, list.out]);
  return out;
}

/** Every face, over a recording `duration` seconds long. `edit` is the trim, the hand cut
 * and the suggestion left unapplied (the voice-cut layer's, later) that the edited faces
 * carry; `markers` its flags. */
export function editorScenes(
  duration,
  { edit = { in: 4, out: duration - 5, cuts: [{ start: 30, end: 42 }, { start: 60, end: 63, applied: false, kind: "line-before-cue" }] }, markers = [18, 55] } = {},
) {
  const edited = { in: edit.in, out: edit.out, cuts: edit.cuts.map((c) => ({ applied: true, kind: "manual", ...c })) };
  const untouched = { in: 0, out: duration, cuts: [] };
  const window = (id, body, { ret = DONE, esc = null } = {}) => ({ id, faces: [id], body, ret, esc, activates: true });
  const body = ({ list = edited, media = { state: "ready", duration }, editable = true, can_undo = list === edited, can_redo = false, footer = FOOTER, footer_tone = "plain", done = button("Done", DONE, "filled") } = {}) => {
    const keep = list ? keepOf(list) : [];
    return {
      title: "Edit — Setting up shroom in two minutes",
      rec: REC,
      media,
      hint: "Drag the yellow ends to trim. Drag across the sound to cut a part.",
      footer,
      footer_tone,
      labels,
      list,
      keep,
      edited_duration: list ? keep.reduce((a, [s, e]) => a + (e - s), 0) : duration,
      source_duration: duration,
      markers,
      editable,
      can_undo,
      can_redo,
      revert: button("Revert to original", { do: "editor-revert" }, "plain", editable && list !== untouched && !!list),
      done,
    };
  };
  const greyed = (label) => button(label, DONE, "filled", false);
  return [
    // A trim, a cut and two markers; Undo reaches back, Redo has nothing.
    { variant: "editor-cut", scene: window("editor", body()) },
    { variant: "editor-untouched", scene: window("editor", body({ list: untouched })) },
    // The record is being read: the timeline waits, Done too.
    { variant: "editor-opening", scene: window("editor", body({ list: null, media: { state: "opening" }, editable: false, done: greyed("Done") }), { ret: null }) },
    {
      variant: "editor-getting",
      scene: window("editor-getting", body({ list: untouched, media: { state: "getting", pct: 40, line: "Getting the video from your space…" }, editable: false, done: greyed("Done") }), { ret: null }),
    },
    {
      variant: "editor-cant-get",
      scene: window(
        "editor-cant-get",
        body({
          list: untouched,
          media: { state: "failed", line: "Couldn't get the video. Check your connection, then try again.", retry: button("Try again", { do: "editor-retry" }, "plain") },
          editable: false,
          done: button("Close", CLOSE, "filled"),
        }),
        { ret: CLOSE, esc: CLOSE },
      ),
    },
    { variant: "editor-saving", scene: window("editor-saving", body({ editable: false, footer: "Saving your edit…", done: greyed("Saving…") }), { ret: null }) },
    {
      variant: "editor-save-failed",
      scene: window("editor-save-failed", body({ footer: "Couldn't save your edit. Check your connection, then press Done again.", footer_tone: "amber" })),
    },
  ].map((e) => ({ window: "editor", ...e }));
}
