// What an edit list plays — a port of crates/shroom-edit's `EditList::checked`, `keep`,
// `edited_duration`, `to_edited`, `to_source` and `is_identity`, held to the same
// vectors (crates/shroom-edit/tests/keep-vectors.json, keep.test.ts). The editor plays the
// core's `keep` as it is; this is for what the window shows before the core has answered:
// a trim being dragged, and the playhead's edited time. Then the playback helpers the
// preview's skip loop runs on a keep (the page's player skips the same way).

/** Nothing shorter plays (seconds): `MIN_KEEP` in shroom-edit. */
export const MIN_KEEP = 1.0;

export type Cut = { start: number; end: number; applied: boolean };
/** The list as the editor sees it: the slice's `list`, with the recording's length. */
export type List = { source_duration: number; in: number; out: number; cuts: Cut[] };
export type Keep = [number, number][];

/** The list made safe to play, as `EditList::checked`. */
export function checked<L extends List>(list: L): L {
  const sd = list.source_duration;
  const d = Number.isFinite(sd) && sd > 0 ? sd : 0;
  const clamp = (t: number) => (Number.isFinite(t) ? Math.min(Math.max(t, 0), d) : NaN);
  let a = clamp(list.in);
  let b = clamp(list.out);
  if (Number.isNaN(a)) a = 0;
  if (Number.isNaN(b)) b = d;
  if (b - a < Math.min(MIN_KEEP, d)) [a, b] = [0, d];
  const cuts = list.cuts.flatMap((c) => {
    const s = clamp(c.start);
    const e = clamp(c.end);
    return Number.isNaN(s) || Number.isNaN(e) || e <= s ? [] : [{ ...c, start: s, end: e }];
  });
  return { ...list, source_duration: d, in: a, out: b, cuts };
}

/** The applied cuts, inside the trim, sorted by their start. */
function applied(list: Omit<List, "source_duration">): Keep {
  return list.cuts
    .filter((c) => c.applied)
    .map((c): [number, number] => [Math.max(c.start, list.in), Math.min(c.end, list.out)])
    .filter(([s, e]) => e > s)
    .sort((x, y) => x[0] - y[0]);
}

/** The source ranges that play, in order: the trim minus the applied cuts, overlaps merged. */
export function keep(list: Omit<List, "source_duration">): Keep {
  const out: Keep = [];
  let at = list.in;
  for (const [s, e] of applied(list)) {
    if (e <= at) continue;
    if (s > at) out.push([at, s]);
    at = Math.max(at, e);
  }
  if (at < list.out) out.push([at, list.out]);
  return out;
}

/** How long a keep plays. */
export const length = (k: Keep) => k.reduce((acc, [s, e]) => acc + (e - s), 0);

/** No shorter part is cut by hand (seconds): `MIN_HAND_CUT` in shroom-edit. */
export const MIN_HAND_CUT = 0.1;

/** How much of the source part `start`–`end` plays: what cutting it would take out. */
export const played = (k: Keep, start: number, end: number) => k.reduce((acc, [s, e]) => acc + Math.max(0, Math.min(e, end) - Math.max(s, start)), 0);

export const editedDuration = (list: Omit<List, "source_duration">) => length(keep(list));

/** A source time in edited time; null in a cut or outside the trim. */
export function keepToEdited(k: Keep, t: number): number | null {
  let acc = 0;
  for (const [s, e] of k) {
    if (t >= s && t < e) return acc + (t - s);
    acc += e - s;
  }
  return null;
}

/** The edited timeline's `e` seconds in, on the source. Past the end, the last kept end;
 * before the start, the first kept start. */
export function keepToSource(k: Keep, e: number): number {
  let acc = 0;
  for (const [s, end] of k) {
    const len = end - s;
    if (e < acc + len) return s + Math.max(e - acc, 0);
    acc += len;
  }
  return k.length ? k[k.length - 1][1] : NaN;
}

export const toEdited = (list: Omit<List, "source_duration">, t: number) => keepToEdited(keep(list), t);
export const toSource = (list: Omit<List, "source_duration">, e: number) => {
  const k = keep(list);
  return k.length ? keepToSource(k, e) : list.out;
};

/** Nothing is left out: the whole recording plays as recorded. */
export function isIdentity(list: List): boolean {
  const k = keep(list);
  return k.length === 1 && k[0][0] <= 0 && k[0][1] >= list.source_duration;
}

// ── playback over a keep ──

/** How near a kept range's end the preview jumps on (seconds): the skip loop looks once a
 * frame, so it leaves a range a little early rather than show a frame of a cut. */
export const END_SLACK = 0.03;
/** A seek lands a hair before the time asked for, sometimes: still in the range. */
const START_SLACK = 0.001;

/** Where playing at source time `t` goes next: `null` while `t` plays, the start of the next
 * kept range when `t` is in a cut, before the trim or at a range's very end, and `"end"`
 * past the last. */
export function nextPlay(k: Keep, t: number): number | "end" | null {
  for (const [s, e] of k) {
    if (t < s - START_SLACK) return s;
    if (t < e - END_SLACK) return null;
  }
  return "end";
}

/** Where a click at source time `t` puts the playhead: there when it plays, else the next
 * kept moment (the last kept end past the last range). */
/** How long the sound eases out before a skip, and back in after it (s): the page's own. */
export const JOIN_EASE = 0.05;

/** The level to play at `t` (0–1): full away from every join; easing to nothing over the
 * last [`JOIN_EASE`] before a skip and back over the first after it, so a cut doesn't click
 * — the share page's `shroomGain`, held to the same cases. The trim's own ends keep it full. */
export function gain(k: Keep, t: number, ease = JOIN_EASE): number {
  let i = 0;
  while (i < k.length && t >= k[i][1] - END_SLACK) i++;
  if (i === k.length) return 1;
  let g = 1;
  if (i < k.length - 1) g = Math.min(g, (k[i][1] - END_SLACK - t) / ease);
  if (i > 0) g = Math.min(g, (t - k[i][0]) / ease);
  return Math.max(0, Math.min(1, g));
}

export function snap(k: Keep, t: number): number {
  for (const [s, e] of k) {
    if (t < s) return s;
    if (t < e) return t;
  }
  return k.length ? k[k.length - 1][1] : t;
}

/** The edited position of source time `t`: in a cut, the moment the edit resumes; past the
 * end, the edited length. */
export function editedAt(k: Keep, t: number): number {
  let acc = 0;
  for (const [s, e] of k) {
    if (t < s) return acc;
    if (t < e) return acc + (t - s);
    acc += e - s;
  }
  return acc;
}
