// The editor's clock reads as the page's: held to worker.js's own `clock` and to the
// player script's `shroomClock` (PLAYER_JS), over a sweep of positions and lengths.
import { describe, expect, it } from "vitest";
import { clock, fill, seconds } from "./clock";

type Page = { clock: (s: number, length?: boolean) => string; PLAYER_JS: string };
// Imported by its URL: worker.js is plain JS with no types, and only this test reads it.
const page: Page = await import(/* @vite-ignore */ new URL("../../../worker/src/worker.js", import.meta.url).href);
const shroomClock = new Function(`${page.PLAYER_JS.match(/function shroomClock\(s,l\)\{[^\n]*\}/)![0]}; return shroomClock;`)() as (s: number, l: boolean) => string;

const times = [0, 0.4, 0.5, 0.99, 1, 7, 7.98, 59.4, 59.5, 59.99, 60, 272.4, 599.5, 3599.4, 3599.5, 3600, 3729, 36000.2, -3, NaN, Infinity];

describe("clock", () => {
  it("reads as the page reads", () => {
    expect([clock(0), clock(7.9), clock(7.98, true), clock(272.4, true), clock(3729), clock(-1), clock(NaN)]).toEqual(["0:00", "0:07", "0:08", "4:32", "1:02:09", "0:00", "0:00"]);
  });
  it("matches worker.js's clock and the player's shroomClock, position and length", () => {
    for (const t of times) {
      for (const length of [false, true]) {
        expect(clock(t, length), `${t} ${length}`).toBe(page.clock(t, length));
        expect(clock(t, length), `${t} ${length}`).toBe(shroomClock(t, length));
      }
    }
  });
  it("fills a Scene template", () => {
    expect(fill("Cut {} s", seconds(12))).toBe("Cut 12.0 s");
    expect(fill("Put back {} s", seconds(3.46))).toBe("Put back 3.5 s");
    expect(fill("was {}", clock(272.4, true))).toBe("was 4:32");
  });
});
