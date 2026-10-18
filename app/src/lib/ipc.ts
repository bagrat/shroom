// The commands between a webview and the core (ARCHITECTURE §3.5). Actions travel back
// exactly as the Scene handed them over.
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

export type Action = { do: string; arg?: unknown };
export type Surface = "popover" | "countdown" | "onboarding" | "library" | "settings" | "feedback" | "editor";

export const act = (surface: Surface, action: Action) => invoke("act", { surface, action });
export const key = (surface: Surface, key: "return" | "esc") => invoke("key", { surface, key });
export const dismiss = () => invoke("dismiss");
// A popover that opened by itself got a key from the person (they clicked into it).
export const keyed = () => invoke("popover_keyed");
export const sceneGet = <T>(surface: Surface) => invoke<T | null>("scene_get", { surface });
// The runner emits each surface's slice to its own label; a listener names its label so it
// hears only that slice (an untargeted listener would hear every surface's).
export const onScene = <T>(f: (slice: T) => void, label: Surface) => listen<T>("scene", (e) => f(e.payload), { target: label });
export const fit = (height: number) => invoke("popover_fit", { height });

/** One step in the editor (N63), times in seconds on the recording's own timeline. The core
 * takes it into the edit's history, or refuses it (a trim that leaves too little, a step
 * while the editor saves): the next slice says what the edit is. */
export type EditStep =
  | { op: "trim"; in: number; out: number }
  | { op: "cut"; start: number; end: number }
  | { op: "put-back"; index: number }
  | { op: "apply"; index: number }
  | { op: "revert" }
  | { op: "undo" }
  | { op: "redo" };
export const edit = (step: EditStep) => invoke("edit", { step });

// A pasted secret goes to the runner and nowhere else: the webview never keeps it (law 13).
// The runner stashes the text, classifies its shape, and the Scene answers.
export const pasteSecret = (field: "token" | "invite", text: string) => invoke("paste_secret", { field, text });
/** A typed field the Scene owns (the link name, the Library's search, a title being
 * renamed, the Members composer's “Who's this for?”). Its input sends from one place,
 * `typed.ts`, which counts the edits it sends and shows the Scene's value back once the
 * Scene has counted them too. */
export type Field = "name" | "library-search" | "title" | "invite-name" | "from-name" | "report-words" | "report-reply-to" | "feedback-words" | "feedback-reply-to";
export const field = (field: Field, value: string) => invoke("field", { field, value });

/** A webview's own errors go to the app's log, so a window that stops drawing says why
 * (CHECKPOINT §10, 3.2). The message only — never a field's contents. */
export function reportErrors(surface: string) {
  const send = (message: string) => void invoke("webview_error", { surface, message: message.slice(0, 500) }).catch(() => {});
  window.addEventListener("error", (e) => send(`${e.message} @ ${e.filename}:${e.lineno}:${e.colno}`));
  window.addEventListener("unhandledrejection", (e) => send(`unhandled: ${String((e.reason as Error)?.stack ?? e.reason)}`));
}
