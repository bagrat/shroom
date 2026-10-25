// The onboarding window's slice, as crates/shroom-core/src/scene.rs serializes it
// (Window<OnboardingBody>). Every string here is final copy from the core; the webview
// renders it and composes nothing.
import type { Action } from "../lib/ipc";
import type { Sheet } from "../library/types";

export type Style = "filled" | "plain" | "quiet" | "red-text" | "red-outline" | "link";
export type Button = { action: Action; label: string; style: Style; enabled: boolean; note: string | null };
// `detail`: what the choice means now, in grey beneath it (System default's device, N44).
export type Choice = { action: Action; label: string; selected: boolean; enabled: boolean; detail?: string };
export type Toggle = { action: Action; label: string; on: boolean; enabled: boolean };

/** A run of a line, plain or in its ratified emphasis; a line's runs join to the line. */
export type Run = { text: string; em: "strong" | "em" | null };
export type Rich = { text: string; runs: Run[] };
/** `uninstall`: quiet at the foot (B160); `sheet`: its question over the window, once asked. */
export type WelcomeBody = { kind: "welcome"; paragraphs: Rich[]; login: Toggle; buttons: Button[]; uninstall: Button; sheet: Sheet | null };

export type Glyph = "current" | "done" | "ahead" | "working" | "stopped";
/** A row and its parts as the core split them (DEVIATIONS #45): the title, the aside under it,
 * a done row's fact at its end. */
export type PlanRow = { glyph: Glyph; line: string; runs: Run[]; title: string; aside: Run[]; fact: string | null; clock: string | null };
/** One part of a wait (N74): its mark, its line, and while it spins its running clock. */
export type Part = { glyph: Glyph; line: string; clock: string | null };

export type Nest =
  | { kind: "account"; question: string; buttons: Button[] }
  | { kind: "signing-up"; line: string; buttons: Button[] }
  | { kind: "token"; line: string; create: Button; placeholder: string; held: boolean; checking: boolean; error: string | null; quiet: string | null }
  | { kind: "pick-account"; line: string; accounts: Choice[]; choose: Button }
  | { kind: "checking" }
  // The card step's check, part by part (N74).
  | { kind: "check"; parts: Part[] }
  // `watch`: once the billing page is open, the watch with its clock; `long`: its quiet
  // line once it has run long (N74).
  | { kind: "gate"; line: string; button: Button | null; quiet: string | null; watch: Part | null; long: string | null }
  | { kind: "name"; prefix: string; value: string; edits: number; suffix: string; line: string; claim: Button; hint: string | null; taken: string | null; unexpected: string | null }
  // `parts`: wiring's parts in the setup; `long`: the name's quiet line from 3 min (N74).
  | { kind: "calm"; line: string; parts: Part[]; long: string | null }
  | { kind: "found"; line: string }
  | { kind: "offline"; line: string }
  // A failure nothing else classifies (DEVIATIONS #25): the resting spinner, its amber line,
  // and Try again.
  | { kind: "unexpected"; line: string; button: Button }
  | { kind: "done"; lines: string[]; allow: Button }
  | { kind: "perm-mic" }
  // macOS's own camera prompt is up (N43): nothing of ours.
  | { kind: "perm-camera" }
  // `settings`: while macOS's own alert leads, “Closed macOS's window?” and its door (N43).
  | { kind: "perm-screen"; line: string; quiet: string; reopen: Button; settings: { quiet: string; button: Button } | null }
  | { kind: "relaunching" }
  | { kind: "priming"; line: string }
  | { kind: "all-set"; lines: string[]; buttons: Button[] }
  // Settings · Space: the plan as the space's settings surface (N36, N42).
  | { kind: "ready"; line: string }
  | { kind: "name-changed"; line: string }
  | { kind: "storage-gone"; line: string; buttons: Button[] }
  | { kind: "screen-off"; button: Button }
  // A Mac without the token, every row ticked: where the space is looked after (an
  // owner-class Mac's line and [ Add a token here ], a member's line alone).
  | { kind: "no-token"; line: string; button: Button | null }
  // Join's answer for good, short of the 200 (N31): under the joining row, its line and
  // the paste door where a new link is the way on.
  | { kind: "join-refused"; line: string; door: string | null };

export type PlanView = { kind: "plan"; header: string; rows: PlanRow[]; current: number | null; nest: Nest };
/** The invite window's faces that aren't a plan (Region 6, N29–N31): member-welcome, the
 * switch, the dead or already-here invite, an arrival waiting on its space. */
export type InviteBody = {
  kind: "invite";
  title: string | null;
  lines: string[];
  /** member-welcome's “Start shroom when you log in” (U4). */
  login: Toggle | null;
  /** Filled action first, as the welcome's twin. */
  buttons: Button[];
  /** A dead invite's paste door: its line. */
  door: string | null;
  /** The spinner rests here. */
  spinner: boolean;
};
export type OnboardingBody = WelcomeBody | PlanView | InviteBody;

export type OnboardingWindow = {
  id: string;
  faces: string[];
  body: OnboardingBody;
  ret: Action | null;
  esc: Action | null;
  activates: boolean;
};

/** A slice is ours only if it is the onboarding window's shape (or null: the window hides). */
export const isOnboarding = (s: unknown): s is OnboardingWindow | null =>
  s === null ||
  (typeof s === "object" && s !== null && "body" in s && ["welcome", "plan", "invite"].includes((s as { body: { kind?: string } }).body?.kind ?? ""));
