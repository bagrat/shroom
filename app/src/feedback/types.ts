// The Send feedback window's slice, as crates/shroom-core/src/scene.rs serializes it
// (Window<FeedbackBody>, N60). Every string is final copy from the core.
import type { Action } from "../lib/ipc";
import type { TextField } from "../library/types";
import type { Button, Toggle } from "../onboarding/types";

export type FeedbackBody = {
  title: string;
  line: string;
  words: TextField;
  reply_label: string;
  reply_to: TextField;
  /** “Include shroom's recent log”, ticked when the window opens, and its line. */
  log: Toggle;
  log_note: string;
  disclosure: string;
  /** What goes, word for word — once it can be told. */
  preview: string | null;
  /** Until then: “Putting it together…”. */
  preparing: string | null;
  failed: string | null;
  mail: Button | null;
  /** Cancel, then Send. */
  buttons: Button[];
  /** shroom.am took it: the form gives way to this. */
  sent: { title: string; line: string; done: Button } | null;
};

export type FeedbackWindow = {
  id: string;
  faces: string[];
  body: FeedbackBody;
  ret: Action | null;
  esc: Action | null;
  activates: boolean;
};

export const isFeedback = (s: unknown): s is FeedbackWindow | null =>
  s === null || (typeof s === "object" && s !== null && "body" in s && typeof (s as { body: { log?: unknown } }).body?.log === "object");
