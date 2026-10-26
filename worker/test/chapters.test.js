// The owner edits the chapters (spec N87, owner-page-chapters; DECISIONS B267): the panel
// Edit chapters opens under the player saves the whole list at once to
// POST /_shroom/chapters/<id>. Once the owner has changed anything the chapters are theirs —
// kept on the recording's clock, carried over trims, cuts and bakes, never written again by
// the space's naming — and the bar shows them on a video of any length, two or more playing.
import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";
import { CHAPTERS_JS, CHAPTER_MIN, COPY, addChapterAt, aiChapters, chaptersPanel, removedChapter, startChapterAt } from "../src/worker.js";
import { M1, MHER, MHER_MAC, NOW, ORIGIN, OWNER_AIR, REC, bodyOf, call, cookieFor, goneKey, ledger, metaKey, metaOf, space } from "./space-fixture.js";

const machine = JSON.parse(readFileSync(new URL("../../spec/machine.json", import.meta.url), "utf8"));
const said = Object.fromEntries(machine.states["owner-page-chapters"].said.map((l) => [l.id.split(".")[1], l.text]));
const { shroomTime } = new Function(`${CHAPTERS_JS};return { shroomTime }`)();

const AI = [[0, "Where we are"], [60, "The budget"], [120, "Next steps"]];
/** A live 180 s video the space named, with three chapters. */
const named = (over = {}) => ({ durationSec: 180, named: { by: "ai", title: "Planning", chapters: AI }, ...over });
/** An edit list over the 180 s recording. */
const edits = (cuts, over = {}) => ({ version: 1, source_duration: 180, in: 0, out: 180, cuts: cuts.map(([start, end], i) => ({ id: `c${i}`, start, end, kind: "manual", applied: true })), rev: 2, ...over });
const mine = (list, over = {}) => ({ by: "owner", at: "2026-10-02T11:00:00.000Z", clock: null, list, off: false, ...over });

const owner = { cookie: cookieFor(M1, OWNER_AIR), origin: ORIGIN };
const PATH = `/_shroom/chapters/${REC}`;
const save = (env, body, headers = owner) => call(env, "POST", PATH, { body, headers });
const record = (env) => env.BUCKET.json(metaKey(REC));
async function video(over = {}) {
  const env = space({ objects: { [metaKey(REC)]: metaOf(REC, { state: "live", ...named(), ...over }) } });
  await ledger(env, REC, M1, OWNER_AIR.id);
  return env;
}
const page = async (env, headers = {}) => { const res = await call(env, "GET", `/${REC}`, { headers }); expect(res.status).toBe(200); return res.text(); };
const barOf = (html) => { const m = html.match(/shroomChapters\(p,(\[\[.*?\]\])\)/); return m ? JSON.parse(m[1]) : null; };

describe("the owner's chapters on the bar", () => {
  it("are shown in place of the space's, on a video of any length, the first at 0", () => {
    expect(aiChapters(named({ chapters: mine([[0, "Intro"], [30, "Demo"]]) }))).toEqual([[0, "Intro"], [30, "Demo"]]);
    // 60 s: under the space's 90 s, still the owner's.
    expect(aiChapters({ durationSec: 60, chapters: mine([[2, "Intro"], [30, "Demo"]]) })).toEqual([[0, "Intro"], [30, "Demo"]]);
    // A list of one, or none, keeps the whole bar — and never falls back to the space's.
    expect(aiChapters(named({ chapters: mine([[0, "Intro"]]) }))).toEqual([]);
    expect(aiChapters(named({ chapters: mine([]) }))).toEqual([]);
  });

  it("switched off, the bar is whole for everyone", () => {
    expect(aiChapters(named({ chapters: mine([[0, "Intro"], [30, "Demo"]], { off: true }) }))).toEqual([]);
  });

  it("follow the edit: a start inside a cut moves to its end, one cut whole is set aside and back with the cut", () => {
    const list = [[0, "A"], [60, "B"], [100, "C"], [150, "D"]];
    const m = (cuts) => named({ chapters: mine(list), edits: edits(cuts) });
    expect(aiChapters(m([[50, 70]]))).toEqual([[0, "A"], [50, "B"], [80, "C"], [130, "D"]]);
    // "B" (60–100) cut whole: set aside, not removed — the record still holds it.
    expect(aiChapters(m([[60, 100]]))).toEqual([[0, "A"], [60, "C"], [110, "D"]]);
    expect(m([[60, 100]]).chapters.list).toHaveLength(4);
    // A cut put back is no longer applied: "B" is back.
    const back = edits([[60, 100]]);
    back.cuts[0].applied = false;
    expect(aiChapters(named({ chapters: mine(list), edits: back }))).toEqual([[0, "A"], [60, "B"], [100, "C"], [150, "D"]]);
    // Left under 5 s by a cut, it hides the same way; the space's 20 s rule isn't theirs.
    expect(aiChapters(m([[62, 98]]))).toEqual([[0, "A"], [64, "C"], [114, "D"]]);
    expect(aiChapters(named({ chapters: mine([[0, "A"], [20, "B"], [32, "C"]]) }))).toEqual([[0, "A"], [20, "B"], [32, "C"]]);
  });

  it("are carried over a bake, as the space's are", () => {
    const AT = "2026-10-04T10:00:00Z";
    const m = named({ durationSec: 160, chapters: mine([[0, "A"], [60, "B"], [100, "C"]]), bakedFrom: [{ at: AT, keep: [[0, 50], [70, 180]] }] });
    expect(aiChapters(m)).toEqual([[0, "A"], [50, "B"], [80, "C"]]);
  });

  it("the space's naming never writes them: a new name and chapters from it don't show", () => {
    const m = named({ chapters: mine([[0, "Mine"], [40, "Also mine"]]) });
    m.named = { by: "ai", title: "Again", chapters: [[0, "X"], [10, "Y"], [90, "Z"]] };
    expect(aiChapters(m)).toEqual([[0, "Mine"], [40, "Also mine"]]);
  });
});

describe("the panel's rows", () => {
  it("start from the space's chapters as the bar shows them, on the recording's clock and the bar's", () => {
    expect(chaptersPanel(named({ edits: edits([[50, 70]]) }))).toEqual({
      mine: false, off: false, clock: null, total: 160,
      rows: [{ s: 0, t: 0, title: "Where we are" }, { s: 70, t: 50, title: "The budget" }, { s: 120, t: 100, title: "Next steps" }],
    });
  });

  it("are the owner's once they're theirs: a chapter cut out whole has no time on the bar", () => {
    const m = named({ chapters: mine([[0, "A"], [60, "B"], [100, "C"]], { off: true }), edits: edits([[60, 100]]) });
    expect(chaptersPanel(m)).toEqual({
      mine: true, off: true, clock: null, total: 140,
      rows: [{ s: 0, t: 0, title: "A" }, { s: 60, t: null, title: "B" }, { s: 100, t: 60, title: "C" }],
    });
  });

  it("are none on a video nobody chaptered", () => {
    expect(chaptersPanel({ durationSec: 40 }).rows).toEqual([]);
  });
});

describe("POST /_shroom/chapters/<id>", () => {
  it("saves the whole list: a kept row by its recording time, a moved or new one by the bar's", async () => {
    const env = await video({ edits: edits([[50, 70]]) });
    const res = await save(env, { clock: null, off: false, list: [{ s: 0, title: "Where we are" }, { t: 45, title: "  The   money " }, { s: 140, title: "Next steps" }, { t: 150, title: "Questions" }] });
    expect(res.status).toBe(200);
    const body = await bodyOf(res);
    // 45 s on the bar is before the cut; 150 s is 170 s on the recording.
    expect(record(env).chapters).toEqual({ by: "owner", at: new Date(NOW).toISOString(), clock: null, off: false, list: [[0, "Where we are"], [45, "The money"], [140, "Next steps"], [170, "Questions"]] });
    expect(body.chapters).toEqual([[0, "Where we are"], [45, "The money"], [120, "Next steps"], [150, "Questions"]]);
    expect(body.panel.mine).toBe(true);
    // The page draws them; the space's are gone from it.
    expect(barOf(await page(env))).toEqual(body.chapters);
  });

  it("switches them off, and on again, keeping the list", async () => {
    const env = await video();
    await save(env, { clock: null, off: true, list: AI.map(([s, title]) => ({ s, title })) });
    expect(record(env).chapters.off).toBe(true);
    expect(barOf(await page(env))).toBeNull();
    await save(env, { clock: null, off: false, list: AI.map(([s, title]) => ({ s, title })) });
    expect(barOf(await page(env))).toEqual(AI);
  });

  it("removing every chapter leaves the whole bar, never the space's again", async () => {
    const env = await video();
    expect((await save(env, { clock: null, off: false, list: [] })).status).toBe(200);
    expect(barOf(await page(env))).toBeNull();
    expect(record(env).named.chapters).toEqual(AI);
  });

  it("refuses a chapter shorter than 5 s, or a time past the end, saving nothing", async () => {
    const env = await video();
    const before = record(env);
    const near = await save(env, { clock: null, off: false, list: [{ s: 0, title: "A" }, { t: 63, title: "B" }, { s: 60, title: "C" }] });
    expect(near.status).toBe(400);
    expect(await bodyOf(near)).toEqual({ why: "short" });
    const end = await save(env, { clock: null, off: false, list: [{ s: 0, title: "A" }, { t: 178, title: "B" }] });
    expect(await bodyOf(end)).toEqual({ why: "short" });
    const past = await save(env, { clock: null, off: false, list: [{ s: 0, title: "A" }, { t: 200, title: "B" }] });
    expect(past.status).toBe(400);
    expect(await bodyOf(past)).toEqual({ why: "outside" });
    expect(record(env)).toEqual(before);
    expect(CHAPTER_MIN).toBe(5);
  });

  it("refuses what isn't a list of named chapters", async () => {
    const env = await video();
    for (const body of [{}, { clock: null, off: false }, { clock: null, off: "no", list: [] }, { clock: null, off: false, list: [{ s: 0, title: "  " }] },
      { clock: null, off: false, list: [{ s: 0 }] }, { clock: null, off: false, list: [{ title: "A" }] }, { clock: null, off: false, list: [{ s: -1, title: "A" }] },
      { clock: null, off: false, list: [{ t: Number.NaN, title: "A" }] }, { clock: null, off: false, list: Array.from({ length: 51 }, (_, i) => ({ s: i * 3, title: "A" })) },
      { clock: null, off: false, list: [{ s: 0, title: "x".repeat(81) }] }]) {
      expect((await save(env, body)).status, JSON.stringify(body)).toBe(400);
    }
  });

  it("refuses a list made on another clock: the editor made the video again since the page loaded", async () => {
    const env = await video({ bakedFrom: [{ at: "2026-10-04T10:00:00Z", keep: [[0, 50], [70, 180]] }] });
    expect((await save(env, { clock: null, off: false, list: [] })).status).toBe(409);
    expect((await save(env, { clock: "2026-10-04T10:00:00Z", off: false, list: [] })).status).toBe(200);
    expect(record(env).chapters.clock).toBe("2026-10-04T10:00:00Z");
  });

  it("is the owner's: no cookie, another member's browser, a deleted video", async () => {
    const env = await video();
    const body = { clock: null, off: false, list: [] };
    expect((await save(env, body, { origin: ORIGIN })).status).toBe(403);
    expect((await save(env, body, { cookie: cookieFor(MHER, MHER_MAC), origin: ORIGIN })).status).toBe(403);
    await env.BUCKET.put(goneKey(REC), "{}");
    expect((await save(env, body)).status).toBe(409);
    expect(record(env).chapters).toBeUndefined();
  });
});

describe("the owner's page", () => {
  const icon = (html) => html.match(/<button class="copy chap"[^>]*>[^<]*<\/button>/)?.[0] ?? "";

  it("has Edit chapters among the title's icons, before the scissors, and the panel's rows; a viewer has neither", async () => {
    const env = await video();
    const html = await page(env, owner);
    expect(icon(html)).toContain(`data-tip="${COPY.editChapters}"`);
    expect(icon(html)).toContain('aria-expanded="false"');
    expect(icon(html)).not.toContain("aria-disabled");
    const head = html.match(/<div class="head">[\s\S]*?<\/div>/)[0];
    expect(head.indexOf('id="chb"')).toBeGreaterThan(head.indexOf('id="dl"'));
    if (head.includes('id="ed"')) expect(head.indexOf('id="chb"')).toBeLessThan(head.indexOf('id="ed"'));
    // Nothing for chapters shows under the player until it's asked for (Bagrat, 2026-10-25).
    expect(html).toContain('<section class="chapters" id="chp" hidden');
    expect(html).toContain("shroomChapterPanel(");
    const viewer = await page(env);
    expect(icon(viewer)).toBe("");
    expect(viewer).not.toContain("shroomChapterPanel(");
  });

  it("before the video plays, the icon is there and greyed", async () => {
    const env = await video({ state: "uploading" });
    expect(icon(await page(env, owner))).toContain('aria-disabled="true"');
  });

  it("a video nobody chaptered still has it, its chapter code ready", async () => {
    const env = await video({ durationSec: 40, named: undefined });
    const html = await page(env, owner);
    expect(icon(html)).not.toBe("");
    expect(html).toContain("function shroomChapters(");
  });
});

describe("the panel's words", () => {
  it("are the spec's", () => {
    expect(COPY.editChapters).toBe(said["edit-chapters"]);
    expect(addChapterAt(80)).toBe(said["add-a-chapter-at"]);
    expect(startChapterAt(80)).toBe(said["start-this-chapter-at"]);
    expect(removedChapter("Trial results and metrics")).toBe(said.removed);
  });

  it("reads a typed time: m:ss, h:mm:ss or seconds", () => {
    expect(shroomTime("1:15")).toBe(75);
    expect(shroomTime(" 1:02:15 ")).toBe(3735);
    expect(shroomTime("0:07.5")).toBe(7.5);
    expect(shroomTime("90")).toBe(90);
    for (const bad of ["", "1:75", "a:10", "1::2", "-3", "1:2:3:4"]) expect(shroomTime(bad), bad).toBeNull();
  });
});
