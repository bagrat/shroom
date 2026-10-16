// The Settings window's slice, as crates/shroom-core/src/scene.rs serializes it
// (Window<SettingsBody>, N36). Every string is final copy from the core.
import type { Action } from "../lib/ipc";
import type { Sheet, TextField } from "../library/types";
import type { Button, Choice, PlanView, Rich, Toggle } from "../onboarding/types";

export type GeneralPane = {
  kind: "general";
  login: Toggle;
  updates_label: string;
  update_line: string;
  update: Button | null;
  whats_new: Button | null;
  /** The help row (B180): “Something off?”, Copy diagnostics, Report a problem…. */
  help_label: string;
  copy_diagnostics: Button;
  report: Button;
  /** After a report went through, until the window closes. */
  sent: string | null;
  /** The line under the card; each of `help_links`' labels is a run of it. */
  help: string;
  help_links: Button[];
  /** “Uninstall shroom…”, quiet text at the tab's foot (B135). */
  uninstall: Button;
};

export type SpacePane = {
  kind: "space";
  plan: PlanView;
  mic: Button | null;
  /** The “made with shroom” switch and its line — owner-class Macs (U3, DEVIATIONS #19). */
  mark: Toggle | null;
  mark_line: string | null;
  exits: Button[];
};

/** A born link's pill: the address up to `/join/` and the stash handle Copy link copies
 * from — never the code itself (law 13). */
export type LinkPill = { prefix: string; link: number; copy: Button; line: string };

/** One Members row: its line, a fact, its buttons, and the link just born under it. */
export type MemberRow = { key: string; line: string; fact: string | null; buttons: Button[]; born: LinkPill | null };

/** The Members tab (N28, N33). */
export type MembersPane = {
  kind: "members";
  title: string;
  line: string;
  you: string;
  add_mac: Button;
  mac_link: LinkPill | null;
  macs: MemberRow[];
  rows: MemberRow[];
  name: TextField;
  invite: Button;
  invite_line: string;
  /** Who invites say they're from, under the composer, and its Change — or, being
   * changed, the field and Save (B158). */
  from:
    | { kind: "shown"; line: Rich; change: Button }
    | { kind: "changing"; label: string; field: TextField; save: Button };
};

/** The Report a problem sheet's form, between its line and its buttons (B180). */
export type ReportForm = {
  words: TextField;
  reply_label: string;
  reply_to: TextField;
  disclosure: string;
  /** What goes, word for word — once gathered. */
  preview: string | null;
  /** Until then: “Putting it together…”. */
  preparing: string | null;
  failed: string | null;
  mail: Button | null;
};

export type SettingsBody = { tabs: Choice[]; pane: GeneralPane | SpacePane | MembersPane; sheet: Sheet | null; report: ReportForm | null };

export type SettingsWindow = {
  id: string;
  faces: string[];
  body: SettingsBody;
  ret: Action | null;
  esc: Action | null;
  activates: boolean;
};

export const isSettings = (s: unknown): s is SettingsWindow | null =>
  s === null ||
  (typeof s === "object" && s !== null && "body" in s && Array.isArray((s as { body: { tabs?: unknown } }).body?.tabs));
