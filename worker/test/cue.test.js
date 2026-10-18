// The judgment above the voice-cue planner (spec N65): mechanism deterministic, judgment
// above. POST /_shroom/api/cue {rec, cue, after, candidates}: for one cue the Mac planned,
// Clef scores whether what was said after it says again everything from each candidate start,
// and the Mac picks. Only the take's maker or an owner-class Mac may ask; whatever Clef can't
// answer (the free day used up, a busy model, a space that doesn't listen) is `later`, and the
// Mac keeps its own plan.
import { describe, expect, it } from "vitest";
import { M1, MHER, MHER_MAC, OWNER_AIR, OWNER_MAC, REC, call, ledger, rateKey, sha, space } from "./space-fixture.js";

const ASK = {
  rec: REC,
  cue: 0,
  after: "To get your video, use this link and you'll see it right away.",
  candidates: ["Click on this link to see it.", "Hi, this is a quick test of the voice cut. Click on this link to see it."],
};

/** Cloudflare's AI as a space that listens holds it (N66), answering as Clef does. */
function clef(answer, calls = []) {
  return {
    run: async (model, input) => {
      calls.push({ model, input });
      if (answer instanceof Error) throw answer;
      return { answers: answer };
    },
  };
}
const scored = (...s) => Object.fromEntries(s.map((n, k) => [`s${k + 1}`, { noul: n }]));
function listening(ai) {
  const env = space();
  env.AI = ai;
  return env;
}
const ask = (env, body, auth) => call(env, "POST", "/_shroom/api/cue", { body, auth });

describe("the judgment above the voice-cue planner (POST /_shroom/api/cue)", () => {
  it("asks Clef once, one question per start, and answers a score for each", async () => {
    const calls = [];
    const env = listening(clef(scored(0.753, 0.033), calls));
    await ledger(env, REC, M1, OWNER_MAC.id);
    const r = await ask(env, ASK, OWNER_MAC.bearer);
    expect(r.status).toBe(200);
    const j = await r.json();
    expect(j).toMatchObject({ shroom: 1, scores: [0.753, 0.033] });
    // How long Clef took, for the Mac's log.
    expect(Number.isInteger(j.ms) && j.ms >= 0).toBe(true);
    expect(calls).toHaveLength(1);
    expect(calls[0].model).toBe("@cf/cloudflare/clef");
    expect(Object.keys(calls[0].input.questions)).toEqual(["s1", "s2"]);
    expect(Object.values(calls[0].input.questions).every((q) => q.type === "noul")).toBe(true);
    // What was said reaches Clef as the state it judges, never as one of its questions.
    expect(calls[0].input.state).toContain(ASK.after);
    for (const c of ASK.candidates) expect(calls[0].input.state).toContain(c);
    expect(JSON.stringify(calls[0].input.questions)).not.toContain(ASK.after);
  });

  it("holds a score out of range to 0–1, and one Clef left out to 0", async () => {
    const env = listening(clef({ s1: { noul: 1.7 } }));
    await ledger(env, REC, M1, OWNER_MAC.id);
    expect(await (await ask(env, ASK, OWNER_MAC.bearer)).json()).toMatchObject({ shroom: 1, scores: [1, 0] });
  });

  it("is the maker's to ask, or an owner-class Mac's", async () => {
    const env = listening(clef(scored(0.9, 0.1)));
    await ledger(env, REC, MHER, MHER_MAC.id);
    expect((await ask(env, ASK, MHER_MAC.bearer)).status).toBe(200);
    expect((await ask(env, ASK, OWNER_AIR.bearer)).status).toBe(200);
    // Someone else's video: a member may not ask about it.
    const theirs = "BbBbBbBbBbBbBbBb";
    await ledger(env, theirs, M1, OWNER_MAC.id);
    expect((await ask(env, { ...ASK, rec: theirs }, MHER_MAC.bearer)).status).toBe(403);
    // No key, no answer; and never from a browser.
    expect((await ask(env, ASK)).status).toBe(401);
    expect((await call(env, "POST", "/_shroom/api/cue", { body: ASK, auth: OWNER_MAC.bearer, headers: { origin: "https://example.com" } })).status).toBe(403);
  });

  it("answers later when Clef can't, with why in a word: the free day used up, a busy model, a space that doesn't listen", async () => {
    const spaces = [
      [listening(clef(new Error("AiError: 4006: you have used up your daily free allocation of 10,000 neurons"))), "daily"],
      [listening(clef(new Error("model busy"))), "busy"],
      [space(), "no-ai"],
    ];
    for (const [env, why] of spaces) {
      await ledger(env, REC, M1, OWNER_MAC.id);
      const r = await ask(env, ASK, OWNER_MAC.bearer);
      expect(r.status).toBe(200);
      expect(await r.json()).toEqual({ shroom: 1, later: true, why });
    }
  });

  it("spends a budget of its own: 120 judgments a Mac an hour, apart from its page keys and uploads", async () => {
    const calls = [];
    const env = listening(clef(scored(0.9, 0.1), calls));
    await ledger(env, REC, M1, OWNER_MAC.id);
    for (let i = 0; i < 120; i++) expect((await ask(env, ASK, OWNER_MAC.bearer)).status).toBe(200);
    const over = await ask(env, ASK, OWNER_MAC.bearer);
    expect(over.status).toBe(429);
    expect(await over.json()).toMatchObject({ error: "too-many" });
    expect(calls).toHaveLength(120);
    expect(env.BUCKET.json(rateKey(OWNER_MAC.id))).toMatchObject({ c: 120, n: 0 });
    // A page key spent after them leaves their count as it was; another Mac still asks.
    expect((await call(env, "POST", "/_shroom/api/key", { body: { kHash: sha("cue-key") }, auth: OWNER_MAC.bearer })).status).toBe(200);
    expect(env.BUCKET.json(rateKey(OWNER_MAC.id))).toMatchObject({ c: 120, k: 1, n: 0 });
    expect((await ask(env, ASK, OWNER_AIR.bearer)).status).toBe(200);
  });

  it("spends nothing it doesn't ask: a space that doesn't listen, a question refused", async () => {
    const env = space();
    await ledger(env, REC, M1, OWNER_MAC.id);
    expect(await (await ask(env, ASK, OWNER_MAC.bearer)).json()).toEqual({ shroom: 1, later: true, why: "no-ai" });
    expect((await ask(env, { ...ASK, cue: -1 }, OWNER_MAC.bearer)).status).toBe(400);
    expect(env.BUCKET.store.has(rateKey(OWNER_MAC.id))).toBe(false);
  });

  it("refuses what isn't a question the planner asks", async () => {
    const calls = [];
    const env = listening(clef(scored(0.9), calls));
    await ledger(env, REC, M1, OWNER_MAC.id);
    const wrong = [
      { ...ASK, rec: "nope" },
      { ...ASK, cue: -1 },
      { ...ASK, cue: 1.5 },
      { ...ASK, after: 7 },
      { ...ASK, after: "x".repeat(2001) },
      { ...ASK, candidates: [] },
      { ...ASK, candidates: Array(7).fill("x") },
      { ...ASK, candidates: ["x".repeat(2001)] },
      { ...ASK, candidates: [3] },
    ];
    for (const body of wrong) expect((await ask(env, body, OWNER_MAC.bearer)).status, JSON.stringify(body).slice(0, 90)).toBe(400);
    expect(calls).toHaveLength(0);
  });
});
