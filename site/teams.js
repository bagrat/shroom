// POST shroom.am/teams: “Tell me when it's ready” from the landing's company sign-in band
// (spec N88, DECISIONS B277). One address, sent once by email to the inbox that reads reports,
// so that person can be told when company sign-in is ready; nothing keeps it on the site.
//
// It comes only from the landing's box, when someone presses the button. The checks run in
// order, and the first that fails answers and sends nothing:
//
//   size     the body is 4 KB at most                                     413 {error:"size"}
//   shape    JSON {email, website?}: one plausible address, 254
//            characters at most                                          400 {error:"shape"}
//   honeypot `website`, a field a person never sees, is empty; filled, it
//            answers as if sent, and sends nothing                        200 {ok:true}
//   switch   TEAMS isn't "off", and the inbox, the binding and the two
//            rate limiters exist                                          503 {error:"off"}
//   rate     three a minute from one address, and a ceiling for all of
//            them (TEAMS_RATE, TEAMS_CEILING: their own pair, so this
//            door never spends the reports' budget)                       429 {error:"rate"}
//
// Then one plain-text message goes out through the `send_email` binding, from
// report-sender@shroom.am to `REPORT_TO` (the same secret and binding as site/report.js),
// Reply-To the address. Nothing is stored: no storage is bound, and request logs are off
// (wrangler.jsonc).

import { FROM, json, message, mime } from "./report.js";

export const MAX_BYTES = 4 * 1024;
export const MAX_EMAIL = 254;
/** The rate limiters this door counts against (site/wrangler.jsonc). */
export const LIMITERS = /** @type {const} */ ({ rate: "TEAMS_RATE", ceiling: "TEAMS_CEILING" });

/** One plausible address: something, one @, then a domain with a dot in it. No spaces, no
 * brackets, quotes or separators, so it is safe in a header as it is. The landing's script
 * holds the same pattern (site/test/teams.test.mjs keeps them equal). */
export const EMAIL = /^[^\s@<>"(),;:\\[\]]+@(?:[^\s@<>"(),;:\\[\]./]+\.)+[^\s@<>"(),;:\\[\]./]{2,}$/;

/** The signup a body holds, or null when its shape is wrong.
 * @param {unknown} body @returns {{ email: string, website: string } | null} */
export function parseSignup(body) {
  if (!body || typeof body !== "object" || Array.isArray(body)) return null;
  const b = /** @type {Record<string, unknown>} */ (body);
  if (Object.keys(b).some((k) => k !== "email" && k !== "website")) return null;
  if (typeof b.email !== "string") return null;
  if (b.website !== undefined && (typeof b.website !== "string" || b.website.length > 200)) return null;
  const email = b.email.trim();
  if (email.length > MAX_EMAIL || !EMAIL.test(email)) return null;
  return { email, website: /** @type {string | undefined} */ (b.website) ?? "" };
}

/** When it was asked, in Armenia time (UTC+4 all year), as the inbox's reader reads it:
 * “30 Oct, 14:52 AMT”. @param {Date} date */
export function asked(date) {
  const t = new Date(date.getTime() + 4 * 3600_000);
  const month = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"][t.getUTCMonth()];
  const hh = String(t.getUTCHours()).padStart(2, "0"), mm = String(t.getUTCMinutes()).padStart(2, "0");
  return `${t.getUTCDate()} ${month}, ${hh}:${mm} AMT`;
}

/** The message's text. @param {string} email @param {Date} date */
export function bodyText(email, date) {
  return `${email} asked to hear when company sign-in is ready.\n\nSent from shroom.am, ${asked(date)}. Nothing else is kept.`;
}

/** POST /teams.
 * @param {Request} request @param {any} env */
export async function handleTeams(request, env) {
  const declared = Number(request.headers.get("content-length") ?? "0");
  if (declared > MAX_BYTES) return json(413, { error: "size" });
  const raw = await request.arrayBuffer();
  if (raw.byteLength > MAX_BYTES) return json(413, { error: "size" });
  let parsed;
  try {
    parsed = JSON.parse(new TextDecoder().decode(raw));
  } catch {
    return json(400, { error: "shape" });
  }
  const s = parseSignup(parsed);
  if (!s) return json(400, { error: "shape" });
  if (s.website !== "") return json(200, { ok: true });

  const rate = env[LIMITERS.rate], ceiling = env[LIMITERS.ceiling];
  if (env.TEAMS === "off" || !env.REPORT_TO || !env.EMAIL || !rate || !ceiling) return json(503, { error: "off" });

  const ip = request.headers.get("cf-connecting-ip") ?? "";
  for (const [limiter, key] of [[rate, ip || "unknown"], [ceiling, "all"]]) {
    const { success } = await limiter.limit({ key });
    if (!success) return json(429, { error: "rate" });
  }

  const date = new Date();
  const raw822 = mime({
    to: env.REPORT_TO,
    name: "shroom.am",
    subject: `Company sign-in: ${s.email}`,
    text: bodyText(s.email, date),
    log: "",
    replyTo: s.email,
    id: crypto.randomUUID(),
    date,
  });
  try {
    await env.EMAIL.send(await message(env, FROM, env.REPORT_TO, raw822));
  } catch (e) {
    // For a live `wrangler tail` only: request logs are off. It names no address.
    console.error(`teams: the binding refused it (${e instanceof Error ? e.message : String(e)})`);
    return json(502, { error: "send" });
  }
  return json(200, { ok: true });
}
