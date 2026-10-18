/// <reference types="vite/client" />
// The editor's port of the edit model, held to the vectors shroom-edit writes
// (crates/shroom-edit/tests/keep-vectors.json): the Rust, the page and the editor all play
// the same list the same way. Then the playback helpers the preview's skip loop runs on.
import { describe, expect, it } from "vitest";
import text from "../../../crates/shroom-edit/tests/keep-vectors.json?raw";
import { checked, edgeRoom, editedAt, editedDuration, END_SLACK, gain, isIdentity, keep, keepToEdited, keepToSource, leavesEnough, length, nextPlay, played, snap, toEdited, toSource, type List } from "./keep";

type Vector = {
  name: string;
  list: List;
  at_source: number[];
  at_edited: number[];
  expect: { keep: [number, number][]; edited_duration: number; identity: boolean; to_edited: (number | null)[]; to_source: number[] };
};
const vectors: Vector[] = JSON.parse(text);
// As the vectors' writer rounds every answer.
const r = (x: number) => Math.round(x * 1e6) / 1e6;

describe("keep-vectors.json", () => {
  it("has the cases", () => expect(vectors.length).toBeGreaterThanOrEqual(10));
  for (const v of vectors) {
    it(v.name, () => {
      const list = checked(v.list);
      expect(keep(list).map(([s, e]) => [r(s), r(e)])).toEqual(v.expect.keep);
      expect(r(editedDuration(list))).toBe(v.expect.edited_duration);
      expect(isIdentity(list)).toBe(v.expect.identity);
      expect(v.at_source.map((t) => toEdited(list, t)).map((x) => (x === null ? null : r(x)))).toEqual(v.expect.to_edited);
      expect(v.at_edited.map((e) => r(toSource(list, e)))).toEqual(v.expect.to_source);
    });
  }
});

const cut = (start: number, end: number, applied = true) => ({ start, end, applied });
const list = (d: number, more: Partial<List> = {}): List => ({ source_duration: d, in: 0, out: d, cuts: [], ...more });

describe("the edit model, as shroom-edit's own tests", () => {
  it("keeps the trim minus applied cuts", () => {
    const l = list(10, { in: 1, out: 9, cuts: [cut(2, 3), cut(2.5, 4), cut(5, 6, false), cut(8.5, 12)] });
    expect(keep(l)).toEqual([[1, 2], [4, 8.5]]);
    expect(editedDuration(l)).toBe(5.5);
    expect(toEdited(l, 4.5)).toBe(1.5);
    expect(toEdited(l, 2.7)).toBeNull();
  });
  it("walks the kept ranges back to the source", () => {
    const l = list(10, { in: 1, cuts: [cut(3, 5)] });
    expect([0, 1.5, 2, 3, 99, -1].map((e) => toSource(l, e))).toEqual([1, 2.5, 5, 6, 10, 1]);
    for (const e of [0, 0.5, 1.99, 2, 4.5, 6.9]) expect(Math.abs(toEdited(l, toSource(l, e))! - e)).toBeLessThan(1e-9);
  });
  it("clamps, and never rejects an end or an overlap", () => {
    const l = checked(list(10, { in: -3, out: 42, cuts: [cut(-1, 0.8), cut(9.5, 11), cut(4, 5), cut(4.6, 5.4), cut(NaN, 2), cut(6, 6)] }));
    expect([l.in, l.out]).toEqual([0, 10]);
    expect(l.cuts.map((c) => [c.start, c.end])).toEqual([[0, 0.8], [9.5, 10], [4, 5], [4.6, 5.4]]);
    expect(keep(l)).toEqual([[0.8, 4], [5.4, 9.5]]);
  });
  it("puts back a trim that leaves nothing; a recording with no length plays nothing", () => {
    expect(keep(checked(list(10, { in: 6, out: 6.2 })))).toEqual([[0, 10]]);
    const l = checked(list(NaN, { out: 3 }));
    expect([l.source_duration, l.in, l.out]).toEqual([0, 0, 0]);
    expect(keep(l)).toEqual([]);
    expect(toSource(l, 4)).toBe(0);
  });
  it("is the identity only when nothing is left out", () => {
    expect(isIdentity(list(10, { cuts: [cut(2, 3, false)] }))).toBe(true);
    expect(isIdentity(list(10, { out: 9.5 }))).toBe(false);
  });
});

describe("a cut's edge, dragged", () => {
  const cuts = [
    { start: 20, end: 28, applied: true },
    { start: 36, end: 40, applied: true },
    { start: 44, end: 49, applied: false },
  ];
  const ends = { in: 3, out: 55 };
  it("stops at the next cut or suggestion, the trim's end, and a tenth of a second from its other edge", () => {
    expect(edgeRoom(cuts, 0, "end", ends)).toEqual([20.1, 36]);
    expect(edgeRoom(cuts, 0, "start", ends)).toEqual([3, 27.9]);
    expect(edgeRoom(cuts, 1, "end", ends)).toEqual([36.1, 44]);
    expect(edgeRoom(cuts, 2, "end", ends)).toEqual([44.1, 55]);
    expect(edgeRoom(cuts, 2, "start", ends)).toEqual([40, 48.9]);
  });
  it("never leaves less than a second playing; a suggestion moved plays nothing new", () => {
    const list = { in: 0, out: 10, cuts: [{ start: 2, end: 8, applied: true }, { start: 8.5, end: 9, applied: false }] };
    expect(leavesEnough(list, 0, 0.3, 9.9, 10)).toBe(false);
    expect(leavesEnough(list, 0, 0.5, 9.5, 10)).toBe(true);
    expect(leavesEnough(list, 0, 1, 9, 10)).toBe(true);
    expect(leavesEnough(list, 1, 0, 10, 10)).toBe(true);
  });
});

describe("what a selection would cut", () => {
  it("is the part of it that plays", () => {
    const k = keep({ in: 2, out: 50, cuts: [{ start: 10, end: 22, applied: true }] });
    expect(played(k, 12, 15)).toBe(0);
    expect(played(k, 9, 23)).toBe(2);
    expect(played(k, 30, 34)).toBe(4);
    expect(played(k, 0, 3)).toBe(1);
  });
});

describe("playback over a keep", () => {
  const k: [number, number][] = [[8, 20], [32, 60]];
  it("plays inside a range and jumps a cut, the trim's in and a range's last moment", () => {
    expect(nextPlay(k, 10)).toBeNull();
    expect(nextPlay(k, 8)).toBeNull();
    expect(nextPlay(k, 7.9995)).toBeNull();
    expect(nextPlay(k, 0)).toBe(8);
    expect(nextPlay(k, 25)).toBe(32);
    expect(nextPlay(k, 20)).toBe(32);
    expect(nextPlay(k, 20 - END_SLACK / 2)).toBe(32);
    expect(nextPlay(k, 59.99)).toBe("end");
    expect(nextPlay(k, 61)).toBe("end");
    expect(nextPlay([], 3)).toBe("end");
  });
  it("snaps a click into what plays", () => {
    expect([0, 9, 25, 32, 61].map((t) => snap(k, t))).toEqual([8, 9, 32, 32, 60]);
  });
  it("reads a source time as edited time", () => {
    expect([0, 8, 10, 25, 32, 40, 61].map((t) => editedAt(k, t))).toEqual([0, 0, 2, 12, 12, 20, 40]);
    expect(length(k)).toBe(40);
    expect(keepToEdited(k, 25)).toBeNull();
    expect(keepToSource(k, 12)).toBe(32);
  });
});

describe("the preview's sound dips at each skip, as the share page's does", () => {
  const K: [number, number][] = [[1, 10], [20, 30], [40, 50]];
  it("plays at full level away from every join, and at the trim's own ends", () => {
    for (const t of [1, 5, 9.8, 25, 45, 49.99, 60]) expect(gain(K, t), `${t}`).toBe(1);
  });
  it("eases out over the last 50 ms before a jump and in over the first 50 ms after", () => {
    expect(gain(K, 9.97 - 0.05)).toBeCloseTo(1, 6);
    expect(gain(K, 9.97 - 0.025)).toBeCloseTo(0.5, 6);
    expect(gain(K, 20)).toBe(0);
    expect(gain(K, 20.025)).toBeCloseTo(0.5, 6);
    expect(gain(K, 20.05)).toBeCloseTo(1, 6);
  });
  it("takes the shorter ease on a range too short for both", () => {
    expect(gain([[0, 5], [6, 6.06], [8, 9]], 6.015)).toBeCloseTo(0.3, 6);
  });
});
