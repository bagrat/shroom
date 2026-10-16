// A typed field the Scene owns (ARCHITECTURE §3.5, DECISIONS B107): the link name, the
// Library's search, a title being renamed, the Members composer's “Who's this for?”.
//
// Every edit goes to the core, and the Scene that answers it arrives a moment later — by
// then more may have been typed, and Scenes arrive for other reasons all the while. So the
// field keeps what's typed, and lets the Scene's value in only once the Scene has counted
// the newest edit sent from here (its `edits`). Then whatever the core refused — a
// character outside the link name's set, a letter past the length — is taken back out,
// the caret where the person has it. With nothing typed and unanswered, the Scene's value
// is the field's, whoever changed it.
//
// How many edits were sent is never forgotten, so an answer to an older edit is known for
// one however late it comes: it never writes over what's typed.
import { field, type Field } from "./ipc";

/** What the Scene says of a typed field: what it holds, and how many edits of it the core
 * has taken in. */
export type Said = { value: string; edits: number };

/** An edit the core never took (its field had just gone) isn't waited for forever: when
 * the core has said something since the last key and, this long on, has counted nothing
 * further, typing gives way to the Scene's value. A core that says nothing — it stalled,
 * or the Mac did — takes nothing back; nor does one whose answers are still coming. */
const UNANSWERED_MS = 2000;

/** `use:typed={{ field, scene }}` on the field's input: nothing else writes its value. */
export function typed(node: HTMLInputElement | HTMLTextAreaElement, p: { field: Field; scene: Said }) {
  let name = p.field;
  let said = p.scene;
  // The count the Scene reaches once it holds everything typed here. It only goes forward
  // — a give-way doesn't take it back — until the window's fields are cleared.
  let sent = said.edits;
  // What's in the field is the person's typing, and stands until the Scene has counted it.
  let standing = false;
  let giveWay: ReturnType<typeof setTimeout> | undefined;

  /** The Scene's value into the field, the caret kept where the person has it. */
  function show(value: string) {
    if (node.value === value) return;
    const has = document.activeElement === node;
    const start = node.selectionStart ?? node.value.length;
    const after = node.value.slice(node.selectionEnd ?? node.value.length);
    node.value = value;
    // Only the field with the keyboard has a caret to keep (and placing one in a field
    // without it would take the keyboard, in WebKit).
    if (!has) return;
    const caret = value.endsWith(after) ? value.length - after.length : Math.min(start, value.length);
    node.setSelectionRange(caret, caret);
  }

  function rest() {
    clearTimeout(giveWay);
    giveWay = undefined;
  }

  /** The Scene's value is the field's — until the next key. */
  function settle() {
    standing = false;
    rest();
    show(said.value);
  }

  function oninput() {
    sent = Math.max(sent, said.edits) + 1;
    standing = true;
    // The wait for this key's answer starts with the next thing the core says.
    rest();
    field(name, node.value);
  }

  node.value = said.value;
  node.addEventListener("input", oninput);

  return {
    update(next: { field: Field; scene: Said }) {
      // A count that went back: the window's fields were cleared, what was typed with them.
      const cleared = next.scene.edits < said.edits;
      const further = next.scene.edits > said.edits;
      name = next.field;
      said = next.scene;
      if (cleared) sent = said.edits;
      // A Scene that hasn't counted the newest edit answers an older one, or something
      // else: typing stands. The give-way runs from the core's first word after the last
      // key, and starts again with every answer that counts further.
      if (standing && said.edits < sent) {
        if (further || giveWay === undefined) {
          rest();
          giveWay = setTimeout(settle, UNANSWERED_MS);
        }
        return;
      }
      settle();
    },
    destroy() {
      node.removeEventListener("input", oninput);
      rest();
    },
  };
}
