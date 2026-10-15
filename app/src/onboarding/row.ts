// A plan row's line, laid out the way the canvas draws a row (DEVIATIONS #45): its title,
// then the aside the line carries in parentheses as the row's small second line, or the
// fact after its dash at the row's right. Nothing is composed or dropped: the parts are
// cut from the Scene's runs where the core split the row (PlanRow's title, aside, fact), and the punctuation itself
// stays in the row as text — drawn as the layout (the second line, the right-hand column),
// kept for anyone who hears the row rather than sees it — so the row still reads, in
// order, exactly the line the Scene gave.
import type { PlanRow, Run } from "./types";

export type RowParts = {
  title: Run[];
  /** The parenthesised aside, without its brackets (`open`/`close` hold them). */
  aside: Run[];
  open: string;
  close: string;
  /** The fact after the dash, without the dash (`dash` holds it). */
  fact: Run[];
  dash: string;
};

/** The runs between two offsets of the line they make. */
function cut(runs: Run[], from: number, to: number): Run[] {
  const out: Run[] = [];
  let at = 0;
  for (const r of runs) {
    const a = Math.max(from, at);
    const b = Math.min(to, at + r.text.length);
    if (b > a) out.push({ text: r.text.slice(a - at, b - at), em: r.em });
    at += r.text.length;
  }
  return out;
}

/** The row's parts, at the places the core split it (its `title`, `aside` and `fact`): the
 * runs keep their emphasis, and the punctuation between the parts is the line's own. */
export function rowParts(row: PlanRow): RowParts {
  const runs = row.runs;
  const line = runs.map((r) => r.text).join("");
  const none = { aside: [], open: "", close: "", fact: [], dash: "" };
  const t = row.title.length;
  const asideText = row.aside.map((r) => r.text).join("");
  if (asideText) {
    const a = line.indexOf(asideText, t);
    if (a >= t) return { ...none, title: cut(runs, 0, t), open: line.slice(t, a), aside: cut(runs, a, a + asideText.length), close: line.slice(a + asideText.length) };
  }
  if (row.fact) {
    const f = line.indexOf(row.fact, t);
    if (f >= t) return { ...none, title: cut(runs, 0, t), dash: line.slice(t, f), fact: cut(runs, f, f + row.fact.length) };
  }
  return { ...none, title: runs };
}
