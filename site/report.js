// POST shroom.am/report: a report or feedback, sent by email to the one inbox that reads them
// (dev/proposals/report-a-problem.md; DECISIONS B180, spec N53).
//
// It comes from the feedback page (shroom.am/feedback) or the app's Report a problem…, only
// when someone presses Send. The checks run in order, and the first that fails answers and
// sends nothing:
//
//   size     the body is 64 KB at most                                    413 {error:"size"}
//   shape    JSON with known fields only, each within its bounds          400 {error:"shape"}
//   switch   REPORTS isn't "off", and the inbox and the binding exist     503 {error:"off"}
//   rate     a few a minute from one address, and a ceiling for everyone  429 {error:"rate"}
//   check    from the page: Turnstile says a person sent it               403 {error:"check"}
//
// Then the log is cleaned once more (site/clean.js), and one message goes out through the
// `send_email` binding: from report-sender@shroom.am (never the inbox's own address — the
// binding refuses a From equal to its To, measured 2026-10-16) to `REPORT_TO`, a Worker secret holding a
// destination verified in Email Routing — never written in this repo. Nothing is stored: no
// storage is bound, and request logs are off (wrangler.jsonc).

import { cleanLog } from "./clean.js";

export const MAX_BYTES = 64 * 1024;
export const FROM = "report-sender@shroom.am";
const SITEVERIFY = "https://challenges.cloudflare.com/turnstile/v0/siteverify";

/** The fields a report may carry, each with its bound in characters. */
const TEXT_FIELDS = /** @type {const} */ ({ words: 8000, name: 80, reply_to: 254, setup: 300, log: 48000, turnstile: 4096 });
const VERSION_FIELDS = ["shroom", "macos", "chip", "memory"];
/** The feedback page's questions, in Bagrat's words (approved 2026-10-16; the one on what
 * was recorded dropped the same day, as too hard to answer): the page shows
 * them, and the message labels each answer with its question. */
export const FEEDBACK_QUESTIONS = [
  "Did setup finish? Where did you stop or have to think?",
  "Did anything break, look wrong, or surprise you?",
  "What did you expect it to do that it didn't?",
  "Would you use it instead of what you use now, or move your team to it? If not, what's the one thing in the way?",
  "Was anything on the site unclear, or hard to believe?",
];
const QUESTIONS = FEEDBACK_QUESTIONS.length;
const ANSWER_CHARS = 4000;

/** @typedef {{ kind: "problem" | "feedback", from: "app" | "page", words?: string, name?: string,
 *   reply_to?: string, setup?: string, log?: string, turnstile?: string,
 *   answers?: string[], versions?: Record<string, string> }} Report */

/** The report a body holds, or null when its shape is wrong.
 * @param {unknown} body @returns {Report | null} */
export function parseReport(body) {
  if (!body || typeof body !== "object" || Array.isArray(body)) return null;
  const b = /** @type {Record<string, unknown>} */ (body);
  const known = new Set(["kind", "from", "answers", "versions", ...Object.keys(TEXT_FIELDS)]);
  if (Object.keys(b).some((k) => !known.has(k))) return null;
  if (b.kind !== "problem" && b.kind !== "feedback") return null;
  if (b.from !== "app" && b.from !== "page") return null;
  for (const [k, max] of Object.entries(TEXT_FIELDS)) {
    if (b[k] !== undefined && (typeof b[k] !== "string" || /** @type {string} */ (b[k]).length > max)) return null;
  }
  if (b.answers !== undefined) {
    if (!Array.isArray(b.answers) || b.answers.length > QUESTIONS) return null;
    if (b.answers.some((a) => typeof a !== "string" || a.length > ANSWER_CHARS)) return null;
  }
  if (b.versions !== undefined) {
    if (!b.versions || typeof b.versions !== "object" || Array.isArray(b.versions)) return null;
    for (const [k, v] of Object.entries(b.versions)) {
      if (!VERSION_FIELDS.includes(k) || typeof v !== "string" || v.length > 80) return null;
    }
  }
  if (b.reply_to && !/^[^\s@<>"]+@[^\s@<>"]+\.[^\s@<>"]+$/.test(/** @type {string} */ (b.reply_to))) return null;
  // Something said: the words, or one answer.
  const said = (typeof b.words === "string" && b.words.trim() !== "") || (Array.isArray(b.answers) && b.answers.some((a) => a.trim() !== ""));
  return said ? /** @type {Report} */ (b) : null;
}

/** One header's text, safe in a header: one line, no angle brackets. @param {string} s */
const oneLine = (s) => s.replace(/[\r\n<>]+/g, " ").trim();

/** The subject line: what it is, which shroom, which macOS. @param {Report} r */
export function subject(r) {
  const what = r.kind === "feedback" ? "Feedback" : "A problem";
  const who = r.name?.trim() ? ` from ${oneLine(r.name).slice(0, 80)}` : "";
  const v = [r.versions?.shroom, r.versions?.macos && `macOS ${r.versions.macos}`].filter(Boolean).map((x) => oneLine(String(x)));
  return `[shroom] ${what}${who}${v.length ? ` · ${v.join(" · ")}` : ""}`;
}

/** The message's text, the log apart (it rides as report.txt).
 * @param {Report} r @param {string[]} questions */
export function bodyText(r, questions) {
  const out = [];
  if (r.name?.trim()) out.push(`From: ${r.name.trim()}`);
  if (r.reply_to) out.push(`Reply to: ${r.reply_to}`);
  if (out.length) out.push("");
  const label = r.kind === "problem" ? "What happened:" : r.from === "app" ? "Feedback:" : "Anything else:";
  if (r.words?.trim()) out.push(label, r.words.trim(), "");
  (r.answers ?? []).forEach((a, i) => {
    if (a.trim()) out.push(`${i + 1}. ${questions[i] ?? `Question ${i + 1}`}`, a.trim(), "");
  });
  const v = r.versions ?? {};
  const mac = [v.shroom && `shroom ${v.shroom}`, v.macos && `macOS ${v.macos}`, v.chip, v.memory].filter(Boolean);
  if (mac.length) out.push(mac.join(" · "));
  if (r.setup?.trim()) out.push(`Set up: ${r.setup.trim()}`);
  out.push(`Sent from: ${r.from === "page" ? "shroom.am/feedback" : r.kind === "feedback" ? "the app (Send feedback)" : "the app (Settings → Report a problem)"}`);
  out.push(r.log?.trim() ? "\nThe log, cleaned, is attached as report.txt." : "\nNo log was sent.");
  return out.join("\n");
}

/** Base64 of UTF-8 text, in 76-character lines. @param {string} s */
function b64(s) {
  const bytes = new TextEncoder().encode(s);
  let bin = "";
  for (const b of bytes) bin += String.fromCharCode(b);
  return btoa(bin).replace(/.{1,76}/g, "$&\r\n").trimEnd();
}

/** An encoded-word for a header that may hold any character. @param {string} s */
const word = (s) => (/^[\x20-\x7e]*$/.test(s) ? s : `=?UTF-8?B?${btoa(String.fromCharCode(...new TextEncoder().encode(s)))}?=`);

/** The whole message, as MIME: the text, and the log attached when there is one.
 * @param {{ to: string, subject: string, text: string, log: string, replyTo?: string, id: string, date: Date }} m */
export function mime(m) {
  const boundary = `shroom-${m.id}`;
  const head = [
    `From: shroom reports <${FROM}>`,
    `To: <${m.to}>`,
    ...(m.replyTo ? [`Reply-To: <${m.replyTo}>`] : []),
    `Subject: ${word(m.subject)}`,
    `Date: ${m.date.toUTCString().replace("GMT", "+0000")}`,
    `Message-ID: <${m.id}@shroom.am>`,
    "MIME-Version: 1.0",
  ];
  const text = ["Content-Type: text/plain; charset=utf-8", "Content-Transfer-Encoding: base64", "", b64(m.text)];
  if (!m.log) return [...head, ...text].join("\r\n") + "\r\n";
  return [
    ...head,
    `Content-Type: multipart/mixed; boundary="${boundary}"`,
    "",
    `--${boundary}`,
    ...text,
    `--${boundary}`,
    'Content-Type: text/plain; charset=utf-8; name="report.txt"',
    'Content-Disposition: attachment; filename="report.txt"',
    "Content-Transfer-Encoding: base64",
    "",
    b64(m.log + "\n"),
    `--${boundary}--`,
    "",
  ].join("\r\n");
}

/** @param {number} status @param {Record<string, unknown>} body */
const json = (status, body) =>
  new Response(JSON.stringify(body), { status, headers: { "content-type": "application/json; charset=utf-8", "cache-control": "no-store" } });

/** Whether Turnstile says the page's sender is a person.
 * @param {string} secret @param {string} token @param {string} ip @param {typeof fetch} fetcher */
async function person(secret, token, ip, fetcher) {
  if (!token) return false;
  const form = new FormData();
  form.set("secret", secret);
  form.set("response", token);
  if (ip) form.set("remoteip", ip);
  try {
    const res = await fetcher(SITEVERIFY, { method: "POST", body: form });
    const out = /** @type {{ success?: boolean }} */ (await res.json());
    return out.success === true;
  } catch {
    return false;
  }
}

/** The binding's message, made: `cloudflare:email`'s EmailMessage on Cloudflare; a test hands
 * its own maker in as `env.EMAIL_MESSAGE`.
 * @param {any} env @param {string} from @param {string} to @param {string} raw */
async function message(env, from, to, raw) {
  const Message = env.EMAIL_MESSAGE ?? (await import("cloudflare:email")).EmailMessage;
  return new Message(from, to, raw);
}

/** POST /report.
 * @param {Request} request @param {any} env @param {typeof fetch} [fetcher] */
export async function handleReport(request, env, fetcher = fetch) {
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
  const r = parseReport(parsed);
  if (!r) return json(400, { error: "shape" });

  if (env.REPORTS === "off" || !env.REPORT_TO || !env.EMAIL) return json(503, { error: "off" });
  if (r.from === "page" && !env.TURNSTILE_SECRET) return json(503, { error: "off" });

  const ip = request.headers.get("cf-connecting-ip") ?? "";
  for (const [limiter, key] of [[env.REPORT_RATE, ip || "unknown"], [env.REPORT_CEILING, "all"]]) {
    if (!limiter) continue;
    const { success } = await limiter.limit({ key });
    if (!success) return json(429, { error: "rate" });
  }

  if (r.from === "page" && !(await person(env.TURNSTILE_SECRET, r.turnstile ?? "", ip, fetcher))) {
    return json(403, { error: "check" });
  }

  const id = crypto.randomUUID();
  const raw822 = mime({
    to: env.REPORT_TO,
    subject: subject(r),
    text: bodyText(r, FEEDBACK_QUESTIONS),
    log: r.log?.trim() ? cleanLog(r.log) : "",
    replyTo: r.reply_to || undefined,
    id,
    date: new Date(),
  });
  try {
    await env.EMAIL.send(await message(env, FROM, env.REPORT_TO, raw822));
  } catch (e) {
    // Why the binding refused, for a live `wrangler tail` only: request logs are off, so
    // nothing keeps it. The message names no part of the report.
    console.error(`report: the binding refused it (${e instanceof Error ? e.message : String(e)})`);
    return json(502, { error: "send" });
  }
  return json(200, { ok: true });
}
