// The Library window's slice, as crates/shroom-core/src/scene.rs serializes it
// (Window<LibraryBody>, N35). Every string is final copy from the core.
import type { Action } from "../lib/ipc";
import type { Button, Choice } from "../onboarding/types";

/** `edits`: how many edits of the field the core has taken in (lib/typed.ts reads it). */
export type TextField = { placeholder: string; value: string; enabled: boolean; edits: number };

/** A destructive sheet (law 9): no Return default, Esc ≡ Cancel, the answer rightmost. */
export type Sheet = { id: string; faces: string[]; title: string; lines: string[]; blocked: string | null; buttons: Button[] };

export type Gestures = {
  click: Action | null;
  cmd_click: Action | null;
  shift_click: Action | null;
  title_click: Action | null;
  double_click: Action | null;
  copy: Action | null;
};

export type Row = {
  key: string;
  title: string | null;
  rename: TextField | null;
  meta: string;
  state: string | null;
  tone: "amber" | "frost" | null;
  progress: number | null;
  pinned: boolean;
  /** Pinned to keep (N74): “Pinned”, said by the small pin before the title. */
  kept: string | null;
  /** Auto-cleanup will delete it (N74): “Deleted Oct 27 unless pinned”, amber. */
  going: string | null;
  selected: boolean;
  gestures: Gestures;
  /** Always visible. */
  buttons: Button[];
  /** On hover or selection. */
  hover: Button[];
  /** The right-click menu. */
  menu: Button[];
  /** *Everyone*'s rows add the member's label (N33): another member's video. */
  member: string | null;
  /** Where the shell serves its poster (`shroom-poster://localhost/<this>`); null: none. */
  poster: string | null;
};

export type LibraryBody = {
  title: string;
  search: TextField;
  /** *Mine · Everyone* — owner-class Macs once someone else has a video here; else empty. */
  scope: Choice[];
  notice: string | null;
  empty: string | null;
  rows: Row[];
  footer: string | null;
  sheet: Sheet | null;
  /** N68's line above the rows: the videos worth deleting, Review… and ✕ (“Not now”). */
  cleanup_line: { line: string; review: Button; dismiss: Button } | null;
  /** Suggested to clean up, in place of the list (N68). */
  cleanup: CleanupBody | null;
  /** Auto-cleanup's amber line, three days ahead (N74), in the suggestion line's place. */
  going_line: Notice | null;
  /** Once a round of auto-cleanup went (N74), with OK. */
  cleaned_line: Notice | null;
};

/** A line above the rows and its buttons (N74). */
export type Notice = { line: string; buttons: Button[] };

/** What auto-cleanup will delete on one date (N74): its heading, a line under the first. */
export type GoingGroup = { heading: string; line: string | null; rows: CleanupRow[] };

/** One suggestion: the poster, the title, then when it was made · its size · why. */
export type CleanupRow = { key: string; title: string | null; meta: string; poster: string | null; going: string | null; buttons: Button[] };

export type CleanupBody = {
  title: string;
  /** “‹ Library”. */
  back: Button;
  intro: string | null;
  /** What auto-cleanup will delete, by date, ahead of the suggestions (N74). */
  going: GoingGroup[];
  /** “Might be worth deleting”, over the suggestions while anything is going (N74). */
  suggest_heading: string | null;
  rows: CleanupRow[];
  /** Once the last suggestion is kept or deleted. */
  empty: string | null;
  foot: string | null;
  /** “Delete all six…”, red text. */
  delete_all: Button | null;
  /** A one-tap delete waiting to be sent. */
  undo: { line: string; undo: Button } | null;
};

/** A card's head, its line, and its two answers (N66's naming card). */

export type LibraryWindow = {
  id: string;
  faces: string[];
  body: LibraryBody;
  ret: Action | null;
  esc: Action | null;
  activates: boolean;
};

export const isLibrary = (s: unknown): s is LibraryWindow | null =>
  s === null ||
  (typeof s === "object" && s !== null && "body" in s && Array.isArray((s as { body: { rows?: unknown } }).body?.rows));
