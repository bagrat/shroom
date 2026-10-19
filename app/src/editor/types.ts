// The editor's slice, as crates/shroom-core/src/scene.rs serializes it (Window<EditorBody>,
// N63). Every string is final copy from the core; the templates (`cut`, `put_back`, `was`)
// carry `{}` where the webview fills in a number it measured.
import type { Action } from "../lib/ipc";
import type { Button } from "../onboarding/types";

export type EditorMedia =
  | { state: "opening" }
  | { state: "getting"; pct: number; line: string }
  | { state: "failed"; line: string; retry: Button }
  | { state: "ready"; duration: number };
/** After a save that leaves something out (B197): the final video being made, its failure,
 * then the question of the original. */
export type EditorAfter =
  | { state: "making"; pct: number; line: string }
  | { state: "failed" }
  | { state: "asking"; title: string; line: string; keep: Button; delete: Button };
/** ✕ with changes not saved (N71): Discard (a click only), Keep editing (Esc), Save (Return). */
export type EditorClosing = { title: string; line: string; discard: Button; keep: Button; save: Button };
export type EditorBody = {
  title: string; rec: string; media: EditorMedia; hint: string;
  footer: string; footer_tone: string;
  /** `time` is “{} / {}”: the edited position, then the edited length; `cut` the chip under
   * a selected part, and under a dashed suggestion (Bagrat, 2026-10-19). */
  labels: { play: string; pause: string; undo: string; redo: string; cut: string; put_back: string; was: string; time?: string };
  list: { in: number; out: number; cuts: { start: number; end: number; applied: boolean; kind: string }[] } | null;
  keep: [number, number][]; edited_duration: number; source_duration: number; markers: number[];
  editable: boolean; can_undo: boolean; can_redo: boolean;
  /** Revert to original; null where the window doesn't offer it (a save the space turned
   * down, B199). */
  revert: Button | null;
  done: Button;
  /** By Revert, while an earlier edit's original is kept: the time it has left. */
  original_kept?: string | null;
  after?: EditorAfter | null;
  closing?: EditorClosing | null;
};
export type EditorWindow = { id: string; faces: string[]; body: EditorBody; ret: Action | null; esc: Action | null; activates: boolean };

declare global {
  interface Window {
    /** Where the editor's video and peaks are served: the shell's `shroom-media:` protocol
     * unless a gallery stands one in. */
    __shroomMediaBase?: string;
  }
}

/** A slice is ours only if it is the editor's shape (or null: the window hides). */
export const isEditor = (s: unknown): s is EditorWindow | null =>
  s === null ||
  (typeof s === "object" && s !== null && "body" in s && typeof (s as { body: { media?: { state?: unknown } } }).body?.media?.state === "string");
