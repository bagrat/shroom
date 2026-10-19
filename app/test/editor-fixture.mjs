// The editor's faces, written by hand as the core will hand them over (the build contract's
// EditorBody, N63) until crates/shroom-core/examples/gallery.rs prints its `editor` rows:
// then the core's slices replace these, entry for entry (`{ window, variant, scene }`, as
// the gallery prints them). The words are the spec's said-lines; the lengths are the test
// video's (editor-preview.mjs makes it), so the slices and the media agree.

export const REC = "k3v9q2mz7x";

const labels = { play: "Play", pause: "Pause", undo: "Undo", redo: "Redo", cut: "Cut {} s", put_back: "Put back {} s", was: "was {}", time: "{} / {}", cut_it_too: "Cut again" };
const button = (label, action, style, enabled = true) => ({ action, label, style, enabled, note: null });
const FOOTER = "Your link stays the same. Viewers see the edit once it's saved.";
const DONE = { do: "editor-done" };
const CLOSE = { do: "editor-close" };
const KEEP = { do: "keep-original" };

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
  const window = (id, body, { ret = DONE, esc = null } = {}) => ({ id, faces: [], body, ret, esc, activates: true });
  const body = ({
    list = edited,
    media = { state: "ready", duration },
    editable = true,
    can_undo = list === edited,
    can_redo = false,
    footer = FOOTER,
    footer_tone = "plain",
    done = button("Done", DONE, "filled"),
    original_kept = null,
    after = null,
    revert = button("Revert to original", { do: "editor-revert" }, "plain", editable && list !== untouched && !!list),
  } = {}) => {
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
      revert,
      done,
      original_kept,
      after,
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
    // The owner page's Edit in shroom asked for another video while this one is open (N69): its
    // amber line in the footer line's place until the next change, everything else as it was.
    { variant: "editor-editing-another", scene: window("editor", body({ footer: "You're editing another video — press Done, then Edit again.", footer_tone: "amber" })) },
    // The space turned the save down (B199): its amber line in the save-failed line's place,
    // no Revert to original, nothing steps, Close in Done's place; Return and Esc are Close.
    {
      variant: "editor-save-refused",
      scene: window(
        "editor-save-refused",
        body({
          editable: false,
          can_undo: false,
          footer: "This video changed since you opened it, so this edit can't be saved.",
          footer_tone: "amber",
          done: button("Close", CLOSE, "filled"),
          revert: null,
        }),
        { ret: CLOSE, esc: CLOSE },
      ),
    },
    // Editing again while an earlier edit's original is kept: the time it has left (B197 (e)),
    // never more than it has (B199), counts in words to nine (law 16) — the core's gallery
    // holds them at 47½ h, 30 h, 20 h, 5½ h, 90 min and 25 min.
    ...[
      ["2-days", "two more days"],
      ["1-day", "one more day"],
      ["hours", "20 more hours"],
      ["hours-in-words", "five more hours"],
      ["1-hour", "one more hour"],
      ["under-an-hour", "less than an hour"],
    ].map(([v, left]) => ({
      variant: `editor-original-kept-${v}`,
      scene: window("editor", body({ original_kept: `The original is kept ${left}` })),
    })),
    // Saved: the final video being made, where the track was; Close leaves it to finish.
    {
      variant: "editor-making",
      scene: window(
        "editor-making",
        body({ editable: false, footer: "The link already plays your edit. You can close this window: shroom finishes it.", done: button("Close", CLOSE, "filled"), after: { state: "making", pct: 40, line: "Making the final video… 40%" } }),
        { ret: CLOSE, esc: CLOSE },
      ),
    },
    {
      variant: "editor-making-failed",
      scene: window(
        "editor-making-failed",
        body({ editable: false, footer: "Couldn't make the final video. The link plays your edit anyway, and shroom tries again later.", footer_tone: "amber", done: button("Close", CLOSE, "filled"), after: { state: "failed" } }),
        { ret: CLOSE, esc: CLOSE },
      ),
    },
    // It's up: “Delete the original?” in the footer's place — Yes, delete leads, never on
    // Return; Esc is Keep for 2 days.
    {
      variant: "editor-keep-original",
      scene: window(
        "editor-keep-original",
        body({
          editable: false,
          done: button("Close", CLOSE, "filled", false),
          after: {
            state: "asking",
            title: "Delete the original?",
            line: "Your edit is the video now. The original is only kept so you can revert to it.",
            keep: button("Keep for 2 days", KEEP, "plain"),
            delete: button("Yes, delete", { do: "delete-original" }, "filled"),
          },
        }),
        { ret: null, esc: KEEP },
      ),
    },
  ].map((e) => ({ window: "editor", ...e }));
}
