// The page's clock (worker.js `clock` and the player's `shroomClock`): 0:07, 4:32,
// 1:02:09. A position counts whole seconds down; a length rounds (a stream that reports
// 7.98 s for an 8 s recording still reads 0:08). clock.test.ts holds it to both.
export function clock(seconds: number, length = false): string {
  const n = Math.max(0, Number(seconds) || 0);
  const s = length ? Math.round(n) : Math.floor(n);
  const h = Math.floor(s / 3600);
  const m = Math.floor((s % 3600) / 60);
  const x = s % 60;
  const two = (v: number) => String(v).padStart(2, "0");
  return h ? `${h}:${two(m)}:${two(x)}` : `${m}:${two(x)}`;
}

/** A Scene template's `{}` filled with what the webview measured (`Cut {} s`, `was {}`). */
export const fill = (template: string, value: string) => template.replace("{}", value);

/** A length in seconds as the cut and put-back chips say it: one decimal. */
export const seconds = (s: number) => Math.max(0, s).toFixed(1);
