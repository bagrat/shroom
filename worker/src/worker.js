// @ts-check
// shroom's Worker — the page behind every link, served from the user's own Cloudflare
// account. One dependency-free ES module (RESEARCH R7.1: provisioning has no bundler);
// the app deploys this exact file. Recordings' bytes and records go straight from the
// recording Mac to the bucket (N4); the Worker reads them, and writes only records of its
// own: the space's roster and ownership ledger, one-time page keys, upload budgets and
// one advisory health flag (WORKER-INTERFACE §4.1).
//
// Routes (ARCHITECTURE §8.1, WORKER-INTERFACE §4):
//   GET /<id>                the page, its face chosen by meta/<id>.json
//   GET /m/<id>/<file>       media from rec/<id>/…, with Range (206) and HEAD
//   GET /d/<id>              the single-file copy, named after the live title
//   GET /_shroom/hls.js      the pinned hls.js, kept in the bucket (no third-party CDN)
//   GET /_shroom/ping        "this answers as a shroom space", with the page's iface and version
//   GET /favicon.svg, /favicon.png, /favicon.ico
//                            the tab icon — the landing's art, carried in this file (no storage read)
//   GET /<id>?k=<key>        redeems a one-time page key → the owner cookie → 302 to the clean URL
//   GET /_shroom/status/<id> the upload line's truth, polled by the page while it waits
//   POST /_shroom/title/<id> a title edit (cookie, checked live; metadata powers only)
//   POST /_shroom/mark       the space-global "made with shroom" switch (owner-class cookie)
//   GET /<id>?embed=1        the embed face: the player alone, the mark small (Notion, iframes)
//   GET /oembed?url=…        oEmbed discovery's answer: an iframe of the embed face
//   GET /join/<code>         the join page and its faces (Region 6, N29)
//   /_shroom/api/…           the app's door for members and Macs without the token:
//                            peek · join · enroll · me · mint · key · library · members ·
//                            invites · invites/<iid> · members/<mid> · devices/<did|self> ·
//                            mark · health — JSON carrying "shroom":1, success or error
//
// Who is who (WORKER-INTERFACE §2, §3): the roster (.shroom/registry/roster.json) holds
// members, Macs and invites, and every change to it is one compare-and-swap. A Mac proves
// itself with its device key (`Bearer <did>.<secret>`, checked against the roster on every
// call); a Mac holding the Cloudflare token enrolls with a parent proof (an HMAC under the
// token's derived S3 secret, the R2_PARENT binding). Owner-ness on pages (DECISIONS B25,
// amended): POST /_shroom/api/key writes .shroom/keys/<sha256(k)> = {member, device, exp};
// redeeming spends it by CAS and sets a cookie {m, d, t} HMAC-signed with
// .shroom/cookie.key, re-checked against the roster on every use. None of .shroom/ is
// ever served: /m/ reads only rec/<id>/…, /d/ only rec/<id>/video.mp4.

/** Said-lines, verbatim from the spec (checked by test/copy.test.js) — or, where the spec
 * prints none, logged in DECISIONS.md's copy table for Bagrat to ratify. */
export const COPY = {
  onItsWay: "This video is on its way. The page refreshes itself — nothing to do.",
  stalled: "This video isn't here yet — the page keeps checking and will play it the moment it arrives.",
  unavailable: "This video can't play right now — try again a little later.",
  // A removed video reads as a plain not found (B141): nothing says one was here.
  notFound: "There's no video at this link.",
  mark: "made with shroom",
  download: "Download",
  copyLink: "Copy link",
  copied: "Copied",
  // Copy thumbnail (N46): the video's picture, wrapped in the link.
  copyThumbnail: "Copy thumbnail",
  preparingThumbnail: "Preparing the thumbnail…",
  waitingConnection: "Waiting for connection — your link already works.",
  waitingSpace: "Waiting for your space — your link already works.",
  // The spec's upload truth ends here, but the page no longer says it: once every file is
  // up, the upload line simply goes (the restyle — Bagrat's call). The popover still does.
  fullyLive: "Fully live ✓",
  // The owner page's upload line, for a few seconds once the last file is up (B142).
  ready: "Ready ✓",
  // The join page (r6: join-landing and its faces).
  getShroom: "Get shroom",
  pasteThis: "Paste this into Terminal (⌘ space, type “Terminal”) and press Return — it installs shroom and opens it, already knowing it's you.",
  copy: "Copy",
  readFirst: "Read it first — the same address opens as readable source.",
  // The Homebrew join line's lead-in (N51).
  useHomebrew: "Use Homebrew? This does the same:",
  // The ratified sentence's shape with the facts that ship: arm64 only (B7; test/sysreq.js).
  macos: "For macOS 14 or newer, Apple silicon.",
  alreadyHave: "Already have shroom?",
  openShroom: "Open shroom",
  openFallback: "If it doesn't open, paste this page's address into shroom.",
  onePerson: "This invite is for one person and works for seven days.",
  elsewhere: "shroom runs on a Mac — open this link on yours to get started.",
  ownHero: "Add this Mac to your shroom space.",
  ownHour: "This link is for one Mac and works for one hour.",
  inviteUnknown: "This invite link doesn't work — ask whoever sent it for a fresh one.",
  ownExpired: "This link has expired — make a fresh one from Members on your other Mac.",
  // Not printed by the spec (r6 gives member wording only): DECISIONS' copy table.
  ownUsed: "This link was already used — make a fresh one from Members on your other Mac.",
  youreIn: "You're in — this invite already did its job. The mushroom in your menu bar is all you need.",
  // New copy (WORKER-INTERFACE §4.3, open question 11), in DECISIONS' copy table.
  inviteTransient: "This invite link doesn't work right now — try again a little later.",
  ownOwnerNote: "This is the page your other Mac will see — opening it here doesn't use it up.",
  // The player's controls are icons; these name them for screen readers and keyboards
  // (Region 4 lists the controls, not their names): DECISIONS' copy table.
  play: "Play",
  pause: "Pause",
  seek: "Seek",
  fullscreen: "Full screen",
  exitFullscreen: "Exit full screen",
  // The restyled player's speed and sound (no control of the spec's): the speed button
  // reads "Playback speed, 1.5×" — these words, then what it shows.
  speed: "Playback speed",
  mute: "Mute",
  unmute: "Unmute",
};

/** The upload line while bytes arrive: "Uploading — 40%. Your link already works." */
/** @param {number} pct */
export const uploadingLine = (pct) => `Uploading — ${pct}%. Your link already works.`;
/** A member's own stalled video while the space can't take uploads (N40, r4). */
/** @param {string} name */
export const waitingFor = (name) => `Waiting for ${name}'s space — your link already works.`;

// The join page's templated lines (r6), the inviter's short name or the space's address in them.
/** @param {string} name */
export const joinHero = (name) => `${name} invited you to record with shroom.`;
/** @param {string} name */
export const joinIntro = (name) => `shroom records your screen and hands you a link to share. Nothing to sign up for — ${name} set up a space for you.`;
/** @param {string} link the join link without its scheme: <host>/join/<code> */
export const installLine = (link) => `curl -fsSL https://shroom.am/install | sh -s -- --join ${link}`;
/** The Homebrew join line (N50, N51): the link whole, since the app takes a web address
 * handed over by `open -a` only as a join link. @param {string} link <host>/join/<code> */
export const brewLine = (link) => `brew install --cask bagrat/shroom/shroom && open -a ~/Applications/shroom.app "https://${link}"`;
/** @param {string} host */
export const ownSame = (host) => `Same videos, same members — ${host}, on this Mac too.`;
/** @param {string} label */
export const ownerNote = (label) => `This is the page ${label} will see — opening it here doesn't use it up.`;
/** @param {string} name */
export const inviteExpired = (name) => `This invite has expired — ask ${name} for a fresh one.`;
/** @param {string} name */
export const inviteUsed = (name) => `This invite was already used. If that wasn't you, ask ${name} for a fresh one.`;

/** The title field's placeholder, the day the recording was made: "Untitled — Aug 21". */
/** @param {string} iso */
export const untitled = (iso) => {
  const d = new Date(iso);
  return `Untitled — ${["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"][d.getUTCMonth()]} ${d.getUTCDate()}`;
};

/** How long an upload may show no progress before the page calls it stalled. */
export const STALL_SECONDS = 120;

/** The page interface (§6.10): routes and fields only grow within an iface. */
export const IFACE = 2;
/** The version of the app that deploys this file — the same as the app's own
 * (`app/src-tauri/tauri.conf.json`, held equal by a shroom-cloud test). */
export const VERSION = "0.1.19";

const ID = /^[A-Za-z0-9_-]{16}$/;
const HLS_JS_KEY = ".shroom/player/hls.min.js";
const COOKIE_KEY = ".shroom/cookie.key";
const MARK_KEY = ".shroom/mark.json";
const OWNER_COOKIE = "shroom_owner";
/** Chrome's ceiling for a cookie's lifetime. */
const COOKIE_SECONDS = 400 * 24 * 3600;
const TITLE_MAX = 200;

/** @type {Record<string, string>} */
const TYPES = {
  m3u8: "application/vnd.apple.mpegurl",
  m4s: "video/iso.segment",
  mp4: "video/mp4",
  // Copy thumbnail's loop (N46, B164): the one kind a page, not a recording Mac, writes here.
  gif: "image/gif",
  jpg: "image/jpeg",
  vtt: "text/vtt",
};

export default {
  /** @param {Request} request @param {Env} env @param {Ctx} ctx */
  async fetch(request, env, ctx) {
    return handle(request, env, Date.now(), ctx);
  },
};

/**
 * @typedef {{ BUCKET: R2Bucket, R2_PARENT?: string }} Env
 * @typedef {{ waitUntil(p: Promise<unknown>): void }} Ctx
 * @typedef {{ etagMatches?: string, etagDoesNotMatch?: string }} R2Conditional
 * @typedef {{ prefix?: string, startAfter?: string, cursor?: string, limit?: number, include?: string[] }} R2ListOptions
 * @typedef {{ objects: R2Object[], truncated: boolean, cursor?: string }} R2Objects
 * @typedef {{ get(key: string, options?: { range?: Headers }): Promise<R2ObjectBody | null>, head(key: string): Promise<R2Object | null>,
 *   put(key: string, value: string | ArrayBufferView, options?: { onlyIf?: R2Conditional, httpMetadata?: { contentType?: string }, customMetadata?: Record<string, string> }): Promise<R2Object | null>,
 *   delete(key: string): Promise<void>, list(options?: R2ListOptions): Promise<R2Objects> }} R2Bucket
 * @typedef {{ member: string, device: string, ownerClass: boolean, roster: any }} Owner
 * @typedef {{ key: string, size: number, etag: string, httpEtag?: string, customMetadata?: Record<string, string>, range?: { offset?: number, length?: number, suffix?: number } }} R2Object
 * @typedef {R2Object & { body: ReadableStream, text(): Promise<string> }} R2ObjectBody
 */

/** `ctx` is the runtime's (its `waitUntil` lets a reply leave before its housekeeping);
 * without one (tests, the local stand-in) that housekeeping is awaited first.
 * @param {Request} request @param {Env} env @param {number} now @param {Ctx} [ctx] */
export async function handle(request, env, now, ctx) {
  const url = new URL(request.url);
  const path = url.pathname;
  if (path.startsWith(API)) return api(request, env, url, now);
  if (request.method === "POST" && (path === "/_shroom/mark" || path.startsWith("/_shroom/title/"))) return post(request, env, url, now);
  if (request.method === "POST" && path.startsWith("/_shroom/thumb/")) return thumbPost(request, env, url);
  const join = path.match(/^\/join\/(.*)$/);
  if (join) return request.method === "GET" || request.method === "HEAD" ? joinPage(request, env, url, join[1], now) : plain(405, "");
  if (request.method !== "GET" && request.method !== "HEAD") return plain(405, "");

  // The stop's reachability probe: "this address answers as a shroom space" — no storage
  // read, so it stays inside the probe's budget (DV #8).
  if (path === "/_shroom/ping") return json({ shroom: true, iface: IFACE, version: VERSION });
  if (path === "/_shroom/hls.js") return media(request, env, HLS_JS_KEY, "text/javascript", "public, max-age=31536000, immutable");
  // The tab icon, answered here so the page below never takes "favicon.ico" for a video's
  // id (a browser asks for that one unprompted) — whatever rides the address.
  if (path === "/favicon.svg" || path === "/favicon.png" || path === "/favicon.ico") return tabIcon(request, path);

  const st = path.match(/^\/_shroom\/status\/([A-Za-z0-9_-]{16})$/);
  // A record that can't be read right now is the unavailable face, never Cloudflare's own
  // error page (DECISIONS B94).
  if (st) return json(await readMeta(env, st[1]).then((meta) => status(meta, now), () => ({ state: "unavailable" })));

  const m = path.match(/^\/m\/([A-Za-z0-9_-]{16})\/([A-Za-z0-9_./-]+)$/);
  if (m) {
    const file = m[2];
    if (file.split("/").some((p) => p === "" || p === "." || p === "..")) return plain(404, "");
    const ext = file.split(".").pop() ?? "";
    // Only the kinds a recording is made of (B125): a recording Mac writes every byte under
    // rec/<id>/, so nothing it names otherwise is served as anything at all.
    const type = TYPES[ext];
    if (!type) return plain(404, "");
    // A tombstoned recording's bytes are gone for everyone (r5: "It's removed
    // everywhere"), even those an interrupted delete or a late segment left behind. The
    // record is read beside the bytes, not before them — first frame stays one round trip.
    const [meta, res] = await Promise.all([
      readMeta(env, m[1]).then((v) => ({ v }), () => null),
      media(request, env, `rec/${m[1]}/${file}`, type, "public, max-age=3600"),
    ]);
    if (!meta || meta.v?.state === "deleted") {
      try { await res.body?.cancel(); } catch { /* nothing to release */ }
      // A record that can't be read can't vouch the video still exists: fail closed.
      return meta ? plain(404, "") : plain(503, "");
    }
    return res;
  }

  const d = path.match(/^\/d\/([A-Za-z0-9_-]{16})$/);
  if (d) return download(request, env, d[1]).catch(() => plain(503, ""));

  if (path === "/oembed") return oembed(env, url);

  const p = path.match(/^\/([^/]+)$/);
  if (p) {
    const k = url.searchParams.get("k");
    if (k !== null) return redeem(request, env, p[1], k, now, ctx);
    // The space's storage failing under the read is the spec's unavailable face (B94).
    if (url.searchParams.get("embed") === "1") return embed(env, p[1], now, url.origin).catch((e) => unavailableEmbed(e));
    // Who this browser is, read beside the record rather than before it (DECISIONS B59 g).
    return page(env, p[1], now, ownerOf(request, env), url.origin).catch((e) => unavailableFace(e));
  }

  return plain(404, "");
}

/** R2's binding errors end in their code, in parentheses — Cloudflare's error-code table
 * (https://developers.cloudflare.com/r2/api/error-codes/), and the local workerd emulator
 * throws the same shape (`get: The specified object name is not valid. (10020)`, probed
 * 2026-10-03, DECISIONS B94). `null` for anything else.
 * @param {unknown} e */
export function storageCode(e) {
  const m = String(/** @type {any} */ (e)?.message ?? "").match(/\((\d{5,6})\)\s*$/);
  return m ? Number(m[1]) : null;
}
/** The codes Cloudflare documents as passing (retry): internal error, temporarily
 * unavailable, client disconnected, too many requests. */
export const STORAGE_TRANSIENT = [10001, 10043, 10054, 10058];

/** Which failures under a page's read mean *unavailable* (r4, N27): **every one**. The
 * record couldn't be read, so nothing can say the video is on its way, removed or gone —
 * and Cloudflare's own error page would be no face of shroom's. The documented passing
 * codes are the ones a space meets; any other (a key R2 refuses, a bug of ours) gets the
 * same honest line rather than a guess. The page refreshes itself, so it plays once the
 * storage answers. @param {unknown} e */
function unavailableFace(e) {
  return why(html(unavailablePage({ mark: true, owner: null, head: "" })), e);
}

/** Names the failure for whoever reads the response's headers (a code, never a message —
 * a message can name a key): `x-shroom-unavailable: storage-10043`, `storage` when R2 gave
 * no code. @param {Response} res @param {unknown} e */
function why(res, e) {
  const code = storageCode(e);
  res.headers.set("x-shroom-unavailable", code === null ? "storage" : `storage-${code}`);
  return res;
}

/** @param {unknown} body */
function json(body, status = 200) {
  return new Response(JSON.stringify(body), { status, headers: { "content-type": "application/json", "x-robots-tag": "noindex", "cache-control": "no-store" } });
}

/** The upload line's truth — no title, so a guessed id learns nothing. */
/** @typedef {{ state: string, pct?: number, stalled?: boolean, poster?: boolean, mp4?: boolean, drained?: boolean }} Status */
/** @param {any} meta @param {number} now @returns {Status} */
export function status(meta, now) {
  if (!meta) return { state: "waiting" };
  if (meta.state === "deleted") return { state: "removed" };
  // The owner page swaps the player in with its first frame, and adds Download when the
  // single-file copy lands after the go-live (B43).
  // Live plays at once; the upload line goes only once the upload drains — the
  // second-class files too, which the Mac counts in `progress.total` and sends after the
  // go-live (r4 `fully-live`: the queue drained; DECISIONS B49). A record without a count is drained.
  if (meta.state === "live" && meta.media?.hls) {
    // `thumb`: Copy thumbnail's newest picture for the title (N46) — what the Library copies.
    // `dur`: the length, which a page opened while the video uploaded draws on its picture.
    const live = { state: "live", poster: !!meta.media.poster, mp4: !!meta.media.mp4, teaser: !!meta.media.teaser, thumb: newestThumb(meta), dur: Number(meta.durationSec) > 0 ? Number(meta.durationSec) : 0, drained: drained(meta) };
    return live.drained ? live : { ...live, pct: pct(meta), stalled: stalledAt(meta, now) };
  }
  if (meta.state !== "uploading") return { state: "unavailable" };
  return { state: "uploading", pct: pct(meta), stalled: stalledAt(meta, now) };
}

/** @param {any} meta */
function pct(meta) {
  const done = meta.progress?.done ?? 0;
  const total = meta.progress?.total ?? 0;
  return total > 0 ? Math.min(99, Math.floor((done * 100) / total)) : 0;
}

/** Every file the Mac counted is up (`progress.done ≥ total`), or it never counted. */
/** @param {any} meta */
function drained(meta) {
  const p = meta.progress;
  return !(p && Number(p.total) > 0 && Number(p.done) < Number(p.total));
}

/** @param {any} meta @param {number} now */
function stalledAt(meta, now) {
  const updated = Date.parse(meta.progress?.updatedAt ?? meta.createdAt ?? "");
  return Number.isFinite(updated) && now - updated > STALL_SECONDS * 1000;
}

// ───────── owner-ness ─────────

/** Redeems a one-time key: the cookie, then the clean URL — the key never rests in the
 * address bar. The grant is spent by a conditional write before anything is issued, so of
 * two concurrent redeems (a prefetch beside the real navigation) exactly one gets a cookie
 * (H8). A spent, expired or unknown key, a lost race, a grant with no device, or a device
 * since removed all land on the clean page the same way, silently. A browser already
 * holding a live owner-class cookie keeps it against a member's grant: a member's `?k=`
 * link must not swap the owner's cookie for theirs (login CSRF) — the grant is spent
 * either way. The reverse always goes through, so no member's link can lock an owner's
 * own key out of a browser it reached first.
 * The redirect waits on one round trip of reads (the grant, the cookie's secret and the
 * roster, together) and the spend — never on the spent grant's delete, which `ctx` finishes
 * after the reply (DECISIONS B59 g: Stop → the owner's page inside TESTING-PLAN §4's 1 s). */
/** @param {Request} request @param {Env} env @param {string} id @param {string} k @param {number} now @param {Ctx} [ctx] */
async function redeem(request, env, id, k, now, ctx) {
  const headers = new Headers({ location: `/${encodeURIComponent(id)}`, "cache-control": "no-store", "referrer-policy": "no-referrer", "x-robots-tag": "noindex" });
  const done = () => new Response(null, { status: 302, headers });
  if (!/^[A-Za-z0-9_-]{32,64}$/.test(k)) return done();
  try {
    const keyName = `${KEYS}${await sha256Hex(k)}`;
    // Storage trouble or no roster reads as null here: the page then opens clean.
    const [object, secret, read] = await Promise.all([
      env.BUCKET.get(keyName),
      cookieSecret(env).catch(() => null),
      readRoster(env).catch(() => null),
    ]);
    if (!object) return done();
    /** @type {any} */
    let g = null;
    try { g = JSON.parse(await object.text()); } catch { g = null; }
    // The token Mac's fallback grant adds `role` (§3.3); it is ignored — the roster decides.
    if (!g || typeof g.member !== "string" || typeof g.device !== "string" || g.spent || !(Date.parse(g.exp) > now)) return done();
    const spent = await casPut(env, keyName, JSON.stringify({ ...g, spent: iso(now) }), { etagMatches: object.etag });
    if (!spent) return done();
    // The spent grant goes once the reply has left (a delete that fails leaves a spent
    // grant, which no redeem accepts).
    const reap = env.BUCKET.delete(keyName).catch(() => {});
    if (ctx) ctx.waitUntil(reap);
    else await reap;
    if (!secret || !read) return done();
    const { roster } = read;
    const granted = checkCookie({ m: g.member, d: g.device }, roster);
    if (!granted) return done();
    if (!granted.ownerClass) {
      /** @type {{ member: string, ownerClass: boolean } | null} */
      let held = null;
      try { held = checkCookie(await cookieClaims(request, env), roster); } catch { held = null; }
      if (held && held.ownerClass) return done();
    }
    const cookie = await signCookie(secret, { m: g.member, d: g.device, t: Math.floor(now / 1000) });
    headers.append("set-cookie", `${OWNER_COOKIE}=${cookie}; Max-Age=${COOKIE_SECONDS}; Path=/; HttpOnly; Secure; SameSite=Lax`);
  } catch {
    // Storage trouble or no roster: the page opens clean, as a recipient's.
  }
  return done();
}

/** The cookie's claims checked against the live roster (§3.5): the issuing device is
 * active and belongs to the member, and the member is active. Owner-class is read from
 * the roster, never from the cookie. */
/** @param {any} claims @param {any} roster @returns {{ member: string, device: string, ownerClass: boolean } | null} */
function checkCookie(claims, roster) {
  if (!claims || typeof claims.m !== "string" || typeof claims.d !== "string") return null;
  const device = own(roster.devices, claims.d);
  const member = own(roster.members, claims.m);
  if (!device || !member || device.member !== claims.m || device.state !== "active" || member.state !== "active") return null;
  return { member: claims.m, device: claims.d, ownerClass: member.role === "owner" };
}

/** The verified cookie's claims, or null — no roster read. */
/** @param {Request} request @param {Env} env */
async function cookieClaims(request, env) {
  const raw = (request.headers.get("cookie") ?? "").split(/;\s*/).find((c) => c.startsWith(`${OWNER_COOKIE}=`));
  if (!raw) return null;
  const secret = await cookieSecret(env);
  if (!secret) return null;
  return verifyCookie(secret, raw.slice(OWNER_COOKIE.length + 1));
}

/** Who this browser is, checked live. A cookie-less request never reads the roster
 * (recipients pay nothing); a cookie reads it once. A missing, broken or unreadable
 * roster makes every cookie a recipient's — pages still render (C2, C8). */
/** @param {Request} request @param {Env} env @returns {Promise<Owner | null>} */
async function ownerOf(request, env) {
  try {
    const claims = await cookieClaims(request, env);
    if (!claims) return null;
    const { roster } = await readRoster(env);
    const who = checkCookie(claims, roster);
    return who ? { ...who, roster } : null;
  } catch {
    return null;
  }
}

/** Whose recording this is (§2.4): the ledger's member; member #1 for an id with a
 * record but no ledger entry (spaces from before the ledger); nobody for neither.
 * `meta.owner` is never read — a member's Mac writes it. */
/** @param {Env} env @param {string} id @param {boolean} hasMeta */
async function ownerOfRecording(env, id, hasMeta) {
  const ledger = await ledgerGet(env, id);
  return ledger ? ledger.member : hasMeta ? M1 : null;
}

/** May this browser rename the recording? Owner-class anything; a member only what the
 * ledger names them for. */
/** @param {Env} env @param {Owner} owner @param {string} id @param {boolean} hasMeta */
async function mayEdit(env, owner, id, hasMeta) {
  if (owner.ownerClass) return true;
  try { return (await ownerOfRecording(env, id, hasMeta)) === owner.member; } catch { return false; }
}

/** @type {Uint8Array<ArrayBuffer> | null} */
let secretCache = null;
/** @param {Env} env */
async function cookieSecret(env) {
  if (secretCache) return secretCache;
  const object = await env.BUCKET.get(COOKIE_KEY);
  if (!object) return null;
  const bytes = fromB64url((await object.text()).trim());
  if (bytes.length < 32) return null;
  secretCache = bytes;
  return bytes;
}
/** Tests reset the per-isolate cache. */
export function forgetSecret() {
  secretCache = null;
}

/** @param {Uint8Array<ArrayBuffer>} secret @param {object} claims */
async function signCookie(secret, claims) {
  const payload = toB64url(new TextEncoder().encode(JSON.stringify(claims)));
  return `${payload}.${toB64url(await hmac(secret, payload))}`;
}

/** @param {Uint8Array<ArrayBuffer>} secret @param {string} value */
async function verifyCookie(secret, value) {
  const [payload, sig] = value.split(".");
  if (!payload || !sig) return null;
  const key = await crypto.subtle.importKey("raw", secret, { name: "HMAC", hash: "SHA-256" }, false, ["verify"]);
  let ok = false;
  try { ok = await crypto.subtle.verify("HMAC", key, fromB64url(sig), new TextEncoder().encode(payload)); } catch { ok = false; }
  if (!ok) return null;
  try {
    const claims = JSON.parse(new TextDecoder().decode(fromB64url(payload)));
    return typeof claims.m === "string" ? claims : null;
  } catch {
    return null;
  }
}

/** @param {Uint8Array<ArrayBuffer>} secret @param {string} payload */
async function hmac(secret, payload) {
  const key = await crypto.subtle.importKey("raw", secret, { name: "HMAC", hash: "SHA-256" }, false, ["sign"]);
  return new Uint8Array(await crypto.subtle.sign("HMAC", key, new TextEncoder().encode(payload)));
}

/** @param {string | Uint8Array<ArrayBuffer>} s */
async function sha256Hex(s) {
  const bytes = typeof s === "string" ? new TextEncoder().encode(s) : s;
  const d = new Uint8Array(await crypto.subtle.digest("SHA-256", bytes));
  return [...d].map((b) => b.toString(16).padStart(2, "0")).join("");
}

/** @param {Uint8Array} bytes */
function toB64url(bytes) {
  let s = "";
  for (const b of bytes) s += String.fromCharCode(b);
  return btoa(s).replace(/\+/g, "-").replace(/\//g, "_").replace(/=+$/, "");
}

/** @param {string} s */
function fromB64url(s) {
  try {
    const bin = atob(s.replace(/-/g, "+").replace(/_/g, "/"));
    return Uint8Array.from(bin, (c) => c.charCodeAt(0));
  } catch {
    return new Uint8Array(0);
  }
}

// ───────── owner writes: metadata only ─────────

/** @param {Request} request @param {Env} env @param {URL} url @param {number} now */
async function post(request, env, url, now) {
  // Same-origin only: the cookie is SameSite=Lax, and the Origin check closes the rest.
  if (request.headers.get("origin") !== url.origin) return plain(403, "");
  const owner = await ownerOf(request, env);
  if (!owner) return plain(403, "");
  /** @type {any} */
  let body = null;
  // The same cap as the API's bodies (§3.8), read through the bounded reader.
  try { body = (await rawBody(request, BODY_MAX)).value; } catch (e) { return plain(e instanceof Refusal ? e.status : 400, ""); }

  const t = url.pathname.match(/^\/_shroom\/title\/([A-Za-z0-9_-]{16})$/);
  if (t) {
    const title = typeof body?.title === "string" ? body.title.replace(/\s+/g, " ").trim().slice(0, TITLE_MAX) : "";
    // An emptied title never saves: the page restores the previous one (N26).
    if (!title) return plain(400, "");
    return writeTitle(env, t[1], title, owner);
  }
  if (url.pathname === "/_shroom/mark") {
    if (!owner.ownerClass || typeof body?.on !== "boolean") return plain(403, "");
    await writeMark(env, body.on, now);
    return json({ on: body.on });
  }
  return plain(404, "");
}

/** Read-modify-write under If-Match (B21): a concurrent rewrite from the recording Mac
 * makes this one retry from a fresh read, never clobber; a tombstone or a deletion
 * record stops it (B125). */
/** @param {Env} env @param {string} id @param {string} title @param {Owner} owner */
async function writeTitle(env, id, title, owner) {
  const key = `meta/${id}.json`;
  /** @type {boolean | null} */
  let allowed = null; // the ledger never changes, so one answer holds for every attempt
  for (let attempt = 0; attempt < 5; attempt++) {
    const [object, gone] = await Promise.all([env.BUCKET.get(key), env.BUCKET.head(`${GONE}${id}`)]);
    if (gone) {
      try { await object?.body?.cancel(); } catch { /* nothing to release */ }
      return plain(409, "");
    }
    if (!object) return plain(404, "");
    /** @type {any} */
    let meta;
    try { meta = JSON.parse(await object.text()); } catch { return plain(409, ""); }
    if (meta.state === "deleted") return plain(409, "");
    if (allowed === null) allowed = await mayEdit(env, owner, id, true);
    if (!allowed) return plain(403, "");
    meta.title = title;
    meta.titled = true;
    const written = await env.BUCKET.put(key, JSON.stringify(meta), { onlyIf: { etagMatches: object.etag }, httpMetadata: { contentType: "application/json" } });
    if (written) return json({ title });
  }
  return plain(503, "");
}

/** Copy thumbnail's caps (N46, B164, B165): a loop 640 wide (the teaser's own width), a still 1200 wide; a title's two
 * pictures, twenty titles' worth. */
export const THUMB = { gifMax: 800 * 1024, jpgMax: 300 * 1024, gifWidth: 640, jpgWidth: 1200, perVideo: 40 };

/** The owner page stores the picture it drew for a title (N46, B164): a title save's
 * checks, then a GIF or a JPEG by its first bytes, under its caps, named by its contents,
 * and pointed at by the record only while the video still has that title. The one byte
 * write a cookie has — never over anything else, never a delete. */
/** @param {Request} request @param {Env} env @param {URL} url */
async function thumbPost(request, env, url) {
  if (request.headers.get("origin") !== url.origin) return plain(403, "");
  const t = url.pathname.match(/^\/_shroom\/thumb\/([A-Za-z0-9_-]{16})$/);
  if (!t) return plain(404, "");
  const id = t[1];
  const owner = await ownerOf(request, env);
  if (!owner) return plain(403, "");
  const kind = request.headers.get("content-type") === "image/gif" ? "gif" : request.headers.get("content-type") === "image/jpeg" ? "jpg" : null;
  if (!kind) return plain(415, "");
  /** @type {Uint8Array<ArrayBuffer>} */
  let bytes;
  try { bytes = await readCapped(request, kind === "gif" ? THUMB.gifMax : THUMB.jpgMax); } catch (e) { return plain(e instanceof Refusal ? e.status : 400, ""); }
  const width = kind === "gif" ? gifWidth(bytes) : jpegWidth(bytes);
  if (width === null) return plain(415, "");
  if (width < 1 || width > (kind === "gif" ? THUMB.gifWidth : THUMB.jpgWidth)) return plain(400, "");
  /** @type {string | null} */
  let drawnFor;
  try { drawnFor = decodeURIComponent(request.headers.get("x-shroom-title") ?? ""); } catch { return plain(400, ""); }
  drawnFor = drawnFor || null;

  const key = `meta/${id}.json`;
  /** @type {boolean | null} */
  let allowed = null;
  let stored = false;
  const name = `thumb-${hex(await crypto.subtle.digest("SHA-256", bytes)).slice(0, 16)}.${kind}`;
  for (let attempt = 0; attempt < 5; attempt++) {
    const [object, gone] = await Promise.all([env.BUCKET.get(key), env.BUCKET.head(`${GONE}${id}`)]);
    if (gone) {
      try { await object?.body?.cancel(); } catch { /* nothing to release */ }
      return plain(409, "");
    }
    if (!object) return plain(404, "");
    /** @type {any} */
    let meta;
    try { meta = JSON.parse(await object.text()); } catch { return plain(409, ""); }
    if (allowed === null) allowed = await mayEdit(env, owner, id, true);
    if (!allowed) return plain(403, "");
    if (meta.state !== "live") return plain(409, "");
    // Drawn for the title the video has now — an untitled video's picture has no band.
    const current = meta.titled ? meta.title : null;
    if (drawnFor !== current) return plain(409, "");
    if (!stored) {
      const listed = await env.BUCKET.list({ prefix: `rec/${id}/thumb-` });
      const have = listed.objects.map((/** @type {{ key: string }} */ o) => o.key);
      if (!have.includes(`rec/${id}/${name}`)) {
        if (have.length >= THUMB.perVideo || listed.truncated) return plain(429, "");
        await env.BUCKET.put(`rec/${id}/${name}`, bytes, { httpMetadata: { contentType: TYPES[kind] ?? "image/jpeg" } });
      }
      stored = true;
    }
    const prev = meta.thumb && typeof meta.thumb === "object" && meta.thumb.title === current ? meta.thumb : {};
    meta.thumb = { ...prev, title: current, [kind]: name };
    const written = await env.BUCKET.put(key, JSON.stringify(meta), { onlyIf: { etagMatches: object.etag }, httpMetadata: { contentType: "application/json" } });
    if (written) return json({ url: `/m/${id}/${name}` });
  }
  return plain(503, "");
}

/** A GIF's logical screen width, or null when the bytes aren't a GIF. */
/** @param {Uint8Array} b */
function gifWidth(b) {
  if (b.length < 14) return null;
  const sig = String.fromCharCode(...b.subarray(0, 6));
  if (sig !== "GIF89a" && sig !== "GIF87a") return null;
  return b[6] | (b[7] << 8);
}

/** A JPEG's width from its first frame header, or null when the bytes aren't a JPEG. */
/** @param {Uint8Array} b */
function jpegWidth(b) {
  if (b.length < 4 || b[0] !== 0xff || b[1] !== 0xd8 || b[2] !== 0xff) return null;
  let i = 2;
  while (i + 9 < b.length) {
    if (b[i] !== 0xff) return null;
    const marker = b[i + 1];
    // The frame headers (SOF0–SOF15 but DHT, JPG and DAC) carry the size.
    if (marker >= 0xc0 && marker <= 0xcf && marker !== 0xc4 && marker !== 0xc8 && marker !== 0xcc) return (b[i + 7] << 8) | b[i + 8];
    i += 2 + ((b[i + 2] << 8) | b[i + 3]);
  }
  return null;
}

/** @param {ArrayBuffer} buf */
function hex(buf) {
  return [...new Uint8Array(buf)].map((x) => x.toString(16).padStart(2, "0")).join("");
}

/** @param {Env} env @param {boolean} on @param {number} now */
async function writeMark(env, on, now) {
  await env.BUCKET.put(MARK_KEY, JSON.stringify({ on, at: iso(now) }), { httpMetadata: { contentType: "application/json" } });
}

/** @param {Env} env */
async function markOn(env) {
  const object = await env.BUCKET.get(MARK_KEY);
  if (!object) return true;
  try { return JSON.parse(await object.text()).on !== false; } catch { return true; }
}

/** @param {number} status @param {string} body */
function plain(status, body) {
  return new Response(body, { status, headers: { "content-type": "text/plain; charset=utf-8", "x-robots-tag": "noindex" } });
}

/** @param {Request} request @param {Env} env @param {string} key @param {string} type @param {string} cache */
async function media(request, env, key, type, cache, extra = {}) {
  // Bytes a recording Mac wrote are never sniffed into something else, and never a page
  // of this origin even opened on their own (B125).
  const headers = new Headers({
    "content-type": type, "accept-ranges": "bytes", "cache-control": cache, "x-robots-tag": "noindex",
    "x-content-type-options": "nosniff", "content-security-policy": "sandbox; default-src 'none'", ...extra,
  });
  if (request.method === "HEAD") {
    const head = await env.BUCKET.head(key);
    if (!head) return plain(404, "");
    headers.set("content-length", String(head.size));
    return new Response(null, { status: 200, headers });
  }
  const range = request.headers.get("range");
  const object = await env.BUCKET.get(key, range ? { range: request.headers } : undefined);
  if (!object) return plain(404, "");
  if (range && object.range) {
    const r = object.range;
    const offset = r.suffix != null ? object.size - r.suffix : (r.offset ?? 0);
    const length = r.suffix != null ? r.suffix : (r.length ?? object.size - offset);
    headers.set("content-range", `bytes ${offset}-${offset + length - 1}/${object.size}`);
    headers.set("content-length", String(length));
    return new Response(object.body, { status: 206, headers });
  }
  headers.set("content-length", String(object.size));
  return new Response(object.body, { status: 200, headers });
}

/** Whether the recording's stream is one picture throughout: its playlist names one init
 * segment. A later take whose picture differs in size has its own (B58 a), and the
 * single-file copy, joined by `-c copy`, can't carry the change. False when the playlist
 * is missing or can't be read: nothing then vouches for the copy. Never throws, and always
 * reads the body through, so the page can start it beside the record and not wait on it.
 * @param {Env} env @param {string} id @returns {Promise<boolean>} */
function onePicture(env, id) {
  if (!ID.test(id)) return Promise.resolve(false);
  return env.BUCKET.get(`rec/${id}/v0/stream.m3u8`)
    .then(async (o) => (o ? (await o.text()).split("#EXT-X-MAP:").length === 2 : false))
    .catch(() => false);
}

/** What the player plays (B177): the single-file copy once it's up and the stream is one
 * picture — a 5-minute view costs ~4–8 of the Worker's requests that way in Chromium, ~47
 * in Safari (it reads a file ~3 MB at a time), against ~56–58 through the stream's 6 s
 * pieces (worker/dev/view-cost.mjs) — and the stream otherwise.
 * Both are served by /m/ under the same removal gate.
 * @param {string} id @param {any} meta @param {Promise<boolean>} oneP */
async function sourceOf(id, meta, oneP) {
  return meta?.state === "live" && meta.media?.mp4 && (await oneP) ? `/m/${id}/video.mp4` : `/m/${id}/master.m3u8`;
}

/** @param {Env} env @param {string} id */
async function readMeta(env, id) {
  if (!ID.test(id)) return null;
  const [object, gone] = await Promise.all([env.BUCKET.get(`meta/${id}.json`), env.BUCKET.head(`${GONE}${id}`)]);
  // A deletion record outranks whatever the meta says now (B125).
  if (gone) {
    try { await object?.body?.cancel(); } catch { /* nothing to release */ }
    return { state: "deleted" };
  }
  if (!object) return null;
  try {
    return JSON.parse(await object.text());
  } catch {
    return null;
  }
}

/** @param {Request} request @param {Env} env @param {string} id */
async function download(request, env, id) {
  const meta = await readMeta(env, id);
  if (!meta || meta.state !== "live" || !meta.media?.mp4) return plain(404, "");
  const name = `${(meta.title || "shroom video").replace(/[\\/:*?"<>|\u0000-\u001f]/g, " ").trim()}.mp4`;
  const disposition = `attachment; filename="${name.replace(/[^\x20-\x7e]/g, "_")}"; filename*=UTF-8''${encodeURIComponent(name)}`;
  // The key is fixed by the id — never taken from the record, which a recording Mac
  // writes: a record naming another key must not turn /d/ into a reader of the bucket.
  return media(request, env, `rec/${id}/video.mp4`, "video/mp4", "private, max-age=0", { "content-disposition": disposition });
}

/** @param {Env} env @param {string} id @param {number} now @param {Promise<Owner | null>} ownerP @param {string} origin */
async function page(env, id, now, ownerP, origin) {
  // Only the record's own read decides the face: a mark that can't be read is the default.
  // The stream's playlist is read beside the record, never after it: whether the copy may
  // play costs no round of its own (B177).
  const oneP = onePicture(env, id);
  const [meta, mark, owner] = await Promise.all([readMeta(env, id), markOn(env).catch(() => true), ownerP]);
  const face = { mark, owner: owner?.ownerClass ? { switchOn: mark } : null, head: meta && meta.state !== "deleted" ? unfurl(id, meta, origin) : "", ratio: ratioOf(meta) };
  if (meta && meta.state === "deleted") return html(notFoundPage(face), false, 404);
  const mine = owner !== null && (await mayEdit(env, owner, id, !!meta));
  // Unknown or malformed ids look exactly like a video still uploading: a guessed link
  // learns nothing, and a link shared before its first byte arrives never 404s (P9).
  if (!meta) {
    // The maker's own browser, before the record's claim lands (the stop's page can open
    // a moment ahead of it): the owner page's own chrome — the title field, Copy link, the
    // upload line — over a record still to come; a title typed meanwhile is saved the
    // moment the record exists (DECISIONS B49). The meta-less row of §4.18 is this face
    // too: when the space can't take uploads, its line says why the video waits.
    // A recording in flight has its ledger entry before its record (§2.4: a token Mac's
    // own mint writes it first, `/mint` does for every other Mac): an id with neither is
    // no recording at all, and reads as one on its way, to anyone.
    // With the space blocked, an owner-class viewer gets the same chrome without a ledger
    // entry (§4.18's “or the viewer is owner-class” row): a token Mac whose token died
    // before the take can't write the entry — its mint is the token's — and N40 still opens
    // that page “as usual, its upload line saying so” (DECISIONS B53).
    if (owner && mine && ID.test(id)) {
      const [ledger, blocked] = await Promise.all([ledgerGet(env, id).catch(() => null), uploadsBlocked(env)]);
      if (ledger || blocked) {
        const pending = { id, state: "uploading", title: "", createdAt: new Date(now).toISOString(), progress: null };
        return html(ownerPage(id, pending, now, face, blocked ? waitingWords(owner) : COPY.waitingConnection, blocked, true));
      }
    }
    return html(onItsWayPage(null, COPY.onItsWay, face, 0, false, true));
  }
  if (owner && mine && (meta.state === "uploading" || (meta.state === "live" && meta.media?.hls))) {
    const blocked = stalledAt(meta, now) && !drained(meta) && (await uploadsBlocked(env));
    return html(ownerPage(id, meta, now, face, blocked ? waitingWords(owner) : COPY.waitingConnection, false, false, await sourceOf(id, meta, oneP)));
  }
  if (meta.state === "uploading") {
    const st = status(meta, now);
    return html(onItsWayPage(meta.title, st.stalled ? COPY.stalled : COPY.onItsWay, face, st.pct ?? 0, !!st.stalled));
  }
  if (meta.state === "live" && meta.media?.hls) return html(livePage(id, meta, face, await sourceOf(id, meta, oneP)));
  return html(unavailablePage(face));
}

/** The owner page's fourth upload face (N40, §4.18): owner-class reads "your space", a
 * member their inviter's — whose name the roster in hand already holds. */
/** @param {Owner} owner */
function waitingWords(owner) {
  if (owner.ownerClass) return COPY.waitingSpace;
  const inviter = own(owner.roster.members, owner.member)?.inviter;
  // Every member record carries its inviter (join copies it from the invite); a record
  // without one keeps the older words rather than invent a name.
  return typeof inviter === "string" && inviter ? waitingFor(inviter) : COPY.waitingConnection;
}

/** Whether the space's uploads are blocked (§4.18's advisory record). Never throws: a
 * page that can't tell says "Waiting for connection", as before. */
/** @param {Env} env */
async function uploadsBlocked(env) {
  try { return (await readHealth(env))?.uploads === "blocked"; } catch { return false; }
}

/** Pages refuse to be framed (the owner's title field and switch live on them); only
 * the embed face may be. */
/** @param {string} body @param {boolean} [frameable] @param {number} [status] */
function html(body, frameable = false, status = 200) {
  return new Response(body, {
    status,
    headers: {
      "content-type": "text/html; charset=utf-8", "x-robots-tag": "noindex", "cache-control": "no-store", "referrer-policy": "no-referrer",
      "content-security-policy": frameable ? "frame-ancestors *" : "frame-ancestors 'none'",
    },
  });
}

// ───────── unfurls (R5.3) ─────────

/** The tags every platform reads, server-rendered (crawlers run no script), og first.
 * og:video is the small teaser, never HLS. No description or site name: nothing the
 * spec has said, and the sender stays anonymous (U3). */
/** @param {string} id @param {any} meta @param {string} origin */
export function unfurl(id, meta, origin) {
  const title = meta.title || "";
  const url = `${origin}/${id}`;
  const m = meta.media ?? {};
  const w = m.width || 0, h = m.height || 0;
  const scaled = (/** @type {number} */ width, even = false) => {
    const v = w && h ? Math.round((width * h) / w) : 0;
    return even ? v - (v % 2) : v;
  };
  const tags = [
    ["og:type", "video.other"],
    ["og:title", title],
    ["og:url", url],
  ];
  if (m.poster) tags.push(["og:image", `${origin}/m/${id}/poster.jpg`], ["og:image:width", "1200"], ["og:image:height", String(scaled(1200))]);
  if (m.teaser) {
    const teaser = `${origin}/m/${id}/teaser.mp4`;
    tags.push(["og:video", teaser], ["og:video:secure_url", teaser], ["og:video:type", "video/mp4"],
      ["og:video:width", "640"], ["og:video:height", String(scaled(640, true))]);
    if (meta.durationSec) tags.push(["og:video:duration", String(Math.round(meta.durationSec))]);
  }
  const named = [["twitter:card", "summary_large_image"], ["twitter:title", title]];
  if (m.poster) named.push(["twitter:image", `${origin}/m/${id}/poster.jpg`]);
  if (m.teaser) named.push(["twitter:player:stream", `${origin}/m/${id}/teaser.mp4`]);
  return tags.map(([k, v]) => `<meta property="${k}" content="${escapeHtml(v)}">`).join("")
    + named.map(([k, v]) => `<meta name="${k}" content="${escapeHtml(v)}">`).join("")
    + `<link rel="alternate" type="application/json+oembed" href="${escapeHtml(`${origin}/oembed?url=${encodeURIComponent(url)}`)}">`;
}

/** oEmbed for this space's own links: an iframe of the embed face. */
/** @param {Env} env @param {URL} url */
async function oembed(env, url) {
  /** @type {URL} */
  let target;
  try { target = new URL(url.searchParams.get("url") ?? ""); } catch { return plain(404, ""); }
  const id = target.pathname.slice(1);
  if (target.origin !== url.origin || !ID.test(id)) return plain(404, "");
  const meta = await readMeta(env, id);
  const live = meta?.state === "live" ? meta : null;
  const w = live?.media?.width || 1280, h = live?.media?.height || 720;
  const width = Math.min(w, 1280), height = Math.round((width * h) / w);
  /** @type {Record<string, unknown>} */
  const body = {
    version: "1.0", type: "video", width, height,
    html: `<iframe src="${escapeHtml(`${url.origin}/${id}?embed=1`)}" width="${width}" height="${height}" frameborder="0" allow="fullscreen; picture-in-picture" allowfullscreen></iframe>`,
  };
  if (live?.title) body.title = live.title;
  if (live?.media?.poster) Object.assign(body, { thumbnail_url: `${url.origin}/m/${id}/poster.jpg`, thumbnail_width: 1200, thumbnail_height: Math.round((1200 * h) / w) });
  return json(body);
}

/** The embed face (§5.4): the player alone, the mark small in its corner; the same
 * on-its-way, unavailable and removed faces; never owner mode. */
/** @param {Env} env @param {string} id @param {number} now @param {string} origin */
async function embed(env, id, now, origin) {
  const oneP = onePicture(env, id);
  const [meta, mark] = await Promise.all([readMeta(env, id), markOn(env).catch(() => true)]);
  // The mark sits at the end of the player's control row (it hides with the controls), or
  // in the corner of a line.
  const corner = mark ? `<a class="mark" href="https://shroom.am" target="_blank" rel="noopener" title="${COPY.mark}" aria-label="${COPY.mark}">${LOGO}</a>` : "";
  const frame = (/** @type {string} */ inner, refresh = 0, code = 200) => html(`<!doctype html><html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1">
<meta name="robots" content="noindex"><title>${escapeText((meta?.state !== "deleted" && meta?.title) || "shroom")}</title>${refresh ? `<meta http-equiv="refresh" content="${refresh}">` : ""}<style>${EMBED_TOKENS}${PLAYER_STYLE}${EMBED_STYLE}</style></head>
<body${mark ? ` class="marked"` : ""}>${inner}${corner}</body></html>`, true, code);
  /** The line where the player will be; on its way, over the thin bar (N27). */
  const line = (/** @type {string} */ text, /** @type {string} */ bar = "") => `<div class="line"><p>${escapeText(text)}</p>${bar}</div>`;
  if (!meta) return frame(line(COPY.onItsWay, thinBar(0, false)) + stallClock(), 5);
  if (meta.state === "deleted") return frame(line(COPY.notFound), 0, 404);
  if (meta.state === "uploading") {
    const st = status(meta, now);
    return frame(line(st.stalled ? COPY.stalled : COPY.onItsWay, thinBar(st.pct ?? 0, !!st.stalled)), 5);
  }
  if (meta.state !== "live" || !meta.media?.hls) return frame(line(COPY.unavailable), 30);
  const src = await sourceOf(id, meta, oneP);
  return frame(`${playerHtml(id, meta, origin)}
<script>${PLAYER_JS};shroomPlayer(document.querySelector(".player"),${JSON.stringify(src)},${jsonForScript(JSON.stringify(LABELS))})</script>`);
}

/** The embed is the player alone, and the player stays dark whatever the viewer's scheme. */
const EMBED_STYLE = `*{box-sizing:border-box}html,body{margin:0;height:100%}body{background:var(--player);color:var(--player-text);font:14px/1.5 -apple-system,BlinkMacSystemFont,system-ui,sans-serif;-webkit-font-smoothing:antialiased}
.line{position:absolute;inset:0;display:flex;flex-direction:column;gap:18px;align-items:center;justify-content:center;margin:0;padding:24px;text-align:center;--track:var(--player-track);--fill:var(--player-fill)}
.line p{margin:0;max-width:420px;font-size:1.0625rem}.line .bar{max-width:320px}
.mark{position:absolute;right:4px;bottom:4px;width:44px;height:44px;display:flex;align-items:center;justify-content:center;border-radius:8px;opacity:.7;transition:opacity .25s}.mark:hover{opacity:1}.mark svg{display:block}
.mark:focus-visible{outline:2px solid var(--player-fill);outline-offset:-2px}
.player{position:absolute;inset:0;border-radius:0}.player video{flex:1 1 0;min-height:0;aspect-ratio:auto}.marked .row{padding-right:52px}.player.idle~.mark{opacity:0}
@media (prefers-reduced-motion:reduce){.mark{transition:none}}`;

/** Text-node escaping: only what can open markup. Apostrophes stay apostrophes. */
/** @param {unknown} s */
export function escapeText(s) {
  return String(s).replace(/[&<>]/g, (c) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;" })[c] ?? c);
}

/** Attribute-value escaping. */
/** @param {unknown} s */
export function escapeHtml(s) {
  return String(s).replace(/[&<>"']/g, (c) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;" })[c] ?? c);
}

/** The player's colours: it stays dark whatever the viewer's scheme, so these are the same
 * under both. */
const PLAYER_TOKENS = `--player:#1f2228;--player-bar:#262a31;--player-text:#d8dee9;--player-hi:#eceff4;--player-track:#434a56;--player-buf:#5e6779;--player-fill:#88c0d0;--player-hover:rgba(236,239,244,.1);--player-wash:rgba(236,239,244,.16)`;

/** The pages' palette, every colour on them: dark Nord, and its light variant for a browser
 * set to light. The embed is the player alone, so it takes the player's colours only. */
const TOKENS = `:root{color-scheme:dark;--bg:#2b2f36;--card:#333944;--card-edge:#333944;--hair:#434a56;--field:#262a31;--well:#262a31;--edge:#4c566a;
--text:#eceff4;--muted:#d8dee9;--soft:#c3cad6;--quiet:#aeb6c4;--accent:#88c0d0;--accent-hi:#8fbcbb;--on-accent:#2b2f36;--hover:rgba(236,239,244,.08);--knob:#eceff4;--stem:#e5e9f0;--green:#a3be8c;--tip:#3b4252;--tip-edge:rgba(236,239,244,.12);${PLAYER_TOKENS}}
@media (prefers-color-scheme:light){:root{color-scheme:light;--bg:#f6f7f9;--card:#fff;--card-edge:#dfe3ea;--hair:#dfe3ea;--field:#fff;--well:#eef1f5;--edge:#c5ccd8;
--text:#2e3440;--muted:#3b4252;--soft:#4c566a;--quiet:#5c6779;--accent:#4c6f9a;--accent-hi:#3f5f86;--on-accent:#fff;--hover:rgba(46,52,64,.06);--knob:#fff;--stem:#b8c0cc;--green:#5d8a45;--tip:#fff;--tip-edge:#dfe3ea}}`;
const EMBED_TOKENS = `:root{color-scheme:dark;${PLAYER_TOKENS}}`;

/** What every page shares: the reset, the type, links, focus, the logo's stem (it would
 * vanish into a light page in its dark-page colour). */
const BASE = `*{box-sizing:border-box}html{-webkit-text-size-adjust:100%}
body{margin:0;min-height:100vh;min-height:100dvh;display:flex;flex-direction:column;align-items:center;background:var(--bg);color:var(--text);font:14px/1.5 -apple-system,BlinkMacSystemFont,system-ui,sans-serif;-webkit-font-smoothing:antialiased}
a{color:var(--accent)}a:hover{color:var(--accent-hi)}:focus-visible{outline:2px solid var(--accent);outline-offset:2px}
.logo .stem{fill:var(--stem,#e5e9f0)}`;

/** An icon drawn by CSS (a mask filled with the text colour), for controls whose words stay
 * their markup: the words name the control, the icon shows it. @param {string} d @param {number} [w] */
const maskIcon = (d, w = 1.4) => `url("data:image/svg+xml,${encodeURIComponent(`<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 16 16' fill='none' stroke='black' stroke-width='${w}' stroke-linecap='round' stroke-linejoin='round'><path d='${d}'/></svg>`)}")`;
const I_LINK = maskIcon("M6.8 9.2a2.8 2.8 0 0 0 4 0l2.2-2.2a2.8 2.8 0 0 0-4-4l-.8.8M9.2 6.8a2.8 2.8 0 0 0-4 0L3 9a2.8 2.8 0 0 0 4 4l.8-.8");
const I_CHECK = maskIcon("M3 8.6l3.2 3.1L13 4.6", 1.6);
const I_SAVE = maskIcon("M8 2.5v8M4.5 7.5L8 11l3.5-3.5M3 13.5h10");
// Copy thumbnail's icon (Bagrat's pick, “7a”): a frame with the play mark, Copy link's chain at its corner.
const I_THUMB = maskIcon("M8.2 12.8H3.3a1.5 1.5 0 0 1-1.5-1.5V4.7a1.5 1.5 0 0 1 1.5-1.5h9.4a1.5 1.5 0 0 1 1.5 1.5v3.5M7 6.2v3.6l3-1.8zM11.89 12.95a1.232 1.232 0 0 0 1.76 0l.968-.968a1.232 1.232 0 0 0-1.76-1.76l-.352.352M12.95 11.89a1.232 1.232 0 0 0-1.76 0L10.22 12.86a1.232 1.232 0 0 0 1.76 1.76l.352-.352");

/** A video's page: the title row (the owner's field, or the title as text) with its quiet
 * icon buttons, the stage, the upload line, and the mark's corner at the foot. */
const STYLE = `${TOKENS}${BASE}
body{padding:40px 16px 24px}
main{width:100%;max-width:960px;flex:1 0 auto;min-height:320px;display:flex;flex-direction:column;gap:20px}
.head{display:flex;align-items:center;gap:4px;min-height:52px}
h1{flex:1 1 0;min-width:0;margin:0;font-size:1.75rem;font-weight:600;line-height:1.25;overflow-wrap:anywhere}
.title{flex:1 1 0;min-width:0;height:52px;margin:0;padding:0 12px;font-family:inherit;font-size:1.75rem;font-weight:600;color:var(--text);background:var(--field);border:1px solid var(--edge);border-radius:9px;outline:none;text-overflow:ellipsis}
.title::placeholder{color:var(--quiet);opacity:1}.title:hover{border-color:var(--quiet)}
.title:not(:focus):not(:placeholder-shown){padding:0;background:transparent;border-color:transparent}
.copy,#dl a,.dl a{flex:none;display:inline-flex;align-items:center;justify-content:center;gap:6px;width:44px;height:44px;padding:0;border:0;border-radius:9px;background:transparent;color:var(--soft);font-family:inherit;font-size:0;text-decoration:none;cursor:pointer;--i:${I_LINK}}
#dl a,.dl a{--i:${I_SAVE}}
.copy.thumb{--i:${I_THUMB}}.copy[hidden]{display:none}
.copy::before,#dl a::before,.dl a::before{content:"";flex:none;width:18px;height:18px;background:currentColor;-webkit-mask:var(--i) center/18px 18px no-repeat;mask:var(--i) center/18px 18px no-repeat}
.copy:hover,#dl a:hover,.dl a:hover{background:var(--hover);color:var(--text)}
.copy[aria-disabled=true]{cursor:default}.copy[aria-disabled=true]::before{opacity:.4}.copy[aria-disabled=true]:hover{background:transparent;color:var(--soft)}
/* The buttons' tooltips (N46: “tooltips need padding between the button”): drawn a clear 8px
   below the button, their right edges on its right edge, so the last one stays in the window. */
.copy,#dl,.dl{position:relative}
[data-tip]::after{content:attr(data-tip);position:absolute;top:calc(100% + 8px);right:0;z-index:5;padding:6px 10px;border-radius:7px;background:var(--tip);border:1px solid var(--tip-edge);color:var(--text);font-size:12.5px;font-weight:400;line-height:1.3;white-space:nowrap;box-shadow:0 6px 18px rgba(0,0,0,.3);opacity:0;visibility:hidden;pointer-events:none;transition:opacity .12s,visibility .12s}
@media (hover:hover){[data-tip]:hover::after{opacity:1;visibility:visible;transition-delay:.3s}}
[data-tip]:focus-visible::after,[data-tip]:has(:focus-visible)::after{opacity:1;visibility:visible}
.copy.done::after{display:none}
.copy.done{width:auto;padding:0 12px 0 8px;flex-direction:row-reverse;font-size:13px;color:var(--accent);--i:${I_CHECK}}
#dl{display:flex}#dl:empty{display:none}
.upl{display:flex;flex-direction:column;gap:6px;color:var(--soft);transition:opacity .4s}.upl[hidden]{display:none}.upl.done{color:var(--green);--fill:var(--green)}.upl.fade{opacity:0}
.wait{position:relative;aspect-ratio:var(--ar);border-radius:12px;background:var(--player);color:var(--player-text);display:flex;flex-direction:column;align-items:center;justify-content:center;gap:18px;padding:24px;text-align:center;overflow:hidden;--track:var(--player-track);--fill:var(--player-fill)}
.wait .line{margin:0;max-width:420px;font-size:1.0625rem}.wait .bar{max-width:320px}
.wait img{position:absolute;inset:0;width:100%;height:100%;object-fit:contain}
main>.line:only-child{margin:auto;max-width:420px;text-align:center;font-size:1.0625rem;color:var(--muted)}
.foot{width:100%;max-width:960px;padding-top:32px;display:flex;justify-content:flex-end;align-items:center;gap:10px;font-size:13px;color:var(--quiet)}.foot[hidden]{display:none}
/* The stage never runs past the window (the canvas's "a short window"): a column holding the
   player or its waiting frame is as wide as the window's height leaves room for at the
   recording's shape (--ar) — the title, the upload line and the mark's foot stay in view —
   and the foot keeps the column's width. 212px: the body's padding, the title row and its
   gap, the foot; a short window's foot sits closer (12px). The player's control bar sits
   under its picture (56px, --bar), so the picture's room is that much less. */
body{--ar:16/9;--room:calc(100dvh - 212px)}
body:has(main .upl:not([hidden])){--room:calc(100dvh - 282px)}
main,body>.foot{--fit:min(960px,max(320px,calc((var(--room) - var(--bar,0px)) * var(--ar))))}
body:has(main .player){--bar:56px}
main:has(.player,.wait),body:has(main .player,main .wait)>.foot{max-width:var(--fit)}
@media (max-height:900px){body{--room:calc(100dvh - 192px)}body:has(main .upl:not([hidden])){--room:calc(100dvh - 262px)}.foot{padding-top:12px}}
.mark{display:flex;align-items:center;gap:8px;min-height:44px;color:var(--quiet);text-decoration:none}.mark:hover{color:var(--text)}.mark[hidden]{display:none}.mark svg{display:block;width:17px;height:18px}
.switch{display:flex;align-items:center;gap:10px;min-height:44px;cursor:pointer}
.switch input{-webkit-appearance:none;appearance:none;flex:none;width:36px;height:20px;margin:0;border-radius:10px;cursor:pointer;background:radial-gradient(circle 8px at 10px 50%,var(--knob) 94%,transparent) no-repeat,var(--edge)}
.switch input:checked{background:radial-gradient(circle 8px at 26px 50%,var(--on-accent) 94%,transparent) no-repeat,var(--accent)}`;

/** The player and the thin bar, shared by the page and the embed face. The bar breathes
 * while bytes move and rests when they stop (N27, N40); on the page it takes the page's
 * colours, in the player's frame the player's. The picture is the recording's whole frame at
 * its own shape (--ar), never cropped and never covered; the control bar sits under it — the
 * seek bar on its top edge (played, buffered, the rest), then play, the time, speed, sound
 * and full screen. Full screen (and the embed's frame) letterbox the picture in what the bar
 * leaves; full screen fades the bar while the video plays and the pointer rests. */
const PLAYER_STYLE = `.bar{display:block;width:100%;height:3px;border-radius:2px;overflow:hidden;background:var(--track,var(--hair))}.bar i{display:block;height:100%;width:0;border-radius:2px;background:var(--fill,var(--accent));transition:width .4s}
.bar.go i{animation:shroom-breathe 1.6s ease-in-out infinite}@keyframes shroom-breathe{50%{opacity:.5}}
.player{position:relative;overflow:hidden;display:flex;flex-direction:column;border-radius:12px;background:var(--player);color:var(--player-text);-webkit-tap-highlight-color:transparent}
.player video{display:block;width:100%;height:auto;aspect-ratio:var(--ar,16/9);object-fit:contain;background:var(--player)}
.player.full video{flex:1 1 0;min-height:0;aspect-ratio:auto}
.big{position:absolute;left:50%;top:calc(50% - 28px);width:72px;height:72px;margin:-36px 0 0 -36px;border-radius:50%;background:var(--player-wash);display:flex;align-items:center;justify-content:center;pointer-events:none;transition:background .15s}
.big svg{display:block;width:26px;height:26px;fill:var(--player-hi)}.player:hover .big{background:var(--player-hover)}.started .big{display:none}
.ctl{position:relative;flex:none;padding-top:4px;background:var(--player-bar);transition:opacity .25s}
.row{height:52px;padding:0 10px;display:flex;align-items:center;gap:4px;font-size:13px}
.row button{flex:none;width:44px;height:44px;padding:0;border:0;border-radius:8px;background:transparent;color:var(--player-text);font:inherit;display:flex;align-items:center;justify-content:center;cursor:pointer}
.row button:hover{background:var(--player-hover);color:var(--player-hi)}.row button:focus-visible{outline:2px solid var(--player-fill);outline-offset:-2px}
.row svg{display:block;width:18px;height:18px;fill:none;stroke:currentColor;stroke-width:1.4;stroke-linecap:round;stroke-linejoin:round}.row .pp svg{fill:var(--player-hi);stroke:none}
.pp .i-pause,.mu .i-off,.fs .i-exit,.playing .pp .i-play,.muted .mu .i-on,.full .fs .i-enter{display:none}
.playing .pp .i-pause,.muted .mu .i-off,.full .fs .i-exit{display:block}
.time{flex:none;padding:0 6px;font-variant-numeric:tabular-nums;white-space:nowrap}.gap{flex:1 1 0}
.row .sp{width:auto;min-width:44px;padding:0 10px;font-weight:600;font-variant-numeric:tabular-nums}
.seek{-webkit-appearance:none;appearance:none;position:absolute;left:0;right:0;top:-6px;width:100%;height:16px;margin:0;padding:0;background:transparent;cursor:pointer;--p:0;--b:0}
.seek::-webkit-slider-runnable-track{height:4px;border:0;background:linear-gradient(to right,var(--player-fill) calc(var(--p)*100%),var(--player-buf) 0 calc(var(--b)*100%),var(--player-track) 0)}
.seek::-moz-range-track{height:4px;border:0;background:linear-gradient(to right,var(--player-fill) calc(var(--p)*100%),var(--player-buf) 0 calc(var(--b)*100%),var(--player-track) 0)}
.seek::-webkit-slider-thumb{-webkit-appearance:none;appearance:none;width:14px;height:14px;margin-top:-5px;border:0;border-radius:50%;background:var(--player-hi)}
.seek::-moz-range-thumb{width:14px;height:14px;border:0;border-radius:50%;background:var(--player-hi)}
.seek:focus-visible{outline:none}.seek:focus-visible::-webkit-slider-thumb{box-shadow:0 0 0 3px var(--player-fill)}.seek:focus-visible::-moz-range-thumb{box-shadow:0 0 0 3px var(--player-fill)}
.player.full.idle .ctl{opacity:0}.player.idle{cursor:none}
.player.full{border-radius:0;width:100%;height:100%}
@media (pointer:coarse){.seek{top:-12px;height:28px}}
@media (max-width:359px){.row .sp{display:none}}
@media (prefers-reduced-motion:reduce){.bar.go i{animation:none}.bar i,.ctl,.big,.upl{transition:none}}`;

/** @typedef {{ mark: boolean, owner: { switchOn: boolean } | null, head: string, ratio?: string }} Face */

/** The recording's shape as CSS reads it (`1024/768`), when its record says. @param {any} meta */
function ratioOf(meta) {
  const m = meta?.media ?? {};
  return m.width > 0 && m.height > 0 ? `${Math.round(m.width)}/${Math.round(m.height)}` : undefined;
}

/** The mark: shroom's logo, small (U3). */
const LOGO = `<svg class="logo" viewBox="18 20 84 90" width="18" height="19" aria-hidden="true"><path class="stem" d="M50,68 C49,82 46,92 46,96 C46,101 52,104 60,104 C68,104 74,101 74,96 C74,92 71,82 70,68 Z" fill="#E5E9F0"/><path d="M24,64 C24,42 40,26 60,26 C80,26 96,42 96,64 C86,75 34,75 24,64 Z" fill="#88C0D0"/><circle cx="33" cy="58" r="3.4" fill="#ECEFF4"/><circle cx="88" cy="58" r="2.2" fill="#ECEFF4"/><circle cx="50" cy="44" r="6" fill="#ECEFF4"/><circle cx="72" cy="48" r="3.6" fill="#ECEFF4"/><circle cx="64" cy="58" r="2.6" fill="#ECEFF4"/></svg>`;

/** The page's bottom-right corner. Anyone's page carries the logo and its words while the
 * space's mark is on, and nothing when it is off. An owner-class browser gets the switch
 * instead of the words — the switch's label is the same three words — beside the logo;
 * and with the mark off the owner's corner is empty too: the switch, its words and the
 * logo all gone, as from everyone's page; Settings turns it back on (DEVIATIONS #41). It
 * stays in the owner's markup, `hidden`, so a write that fails can bring it back. */
/** @param {Face} face */
function foot(face) {
  if (face.owner) {
    const on = face.owner.switchOn;
    const icon = `<a class="mark" href="https://shroom.am" title="${COPY.mark}" aria-label="${COPY.mark}"${on ? "" : " hidden"}>${LOGO}</a>`;
    return `<div class="foot"${on ? "" : " hidden"}><label class="switch"><input type="checkbox" id="mk"${on ? " checked" : ""}>${COPY.mark}</label>${icon}</div>`;
  }
  return face.mark ? `<div class="foot"><a class="mark" href="https://shroom.am" title="${COPY.mark}">${LOGO}<span>${COPY.mark}</span></a></div>` : "";
}

/** @param {string} title @param {string} inner @param {Face} face @param {string} [head] @param {string} [script] */
function shell(title, inner, face, head = "", script = "") {
  return `<!doctype html><html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1">
<meta name="robots" content="noindex">${face.head}<title>${escapeText(title)}</title>${head}${TAB_ICON_LINKS}<style>${STYLE}${PLAYER_STYLE}</style></head>
<body${face.ratio ? ` style="--ar:${face.ratio}"` : ""}><main>${inner}</main>${foot(face)}${face.owner ? MARK_SCRIPT : ""}${script}</body></html>`;
}

/** The owner's switch (U3, N26; DEVIATIONS #41). What the page rendered is what the space
 * says, so the switch starts from that — never from a state the browser put back into the
 * control on a Back, whether it did so before this ran or after (Chromium does it after,
 * just ahead of `load`, with no change event — hence `pageshow`). `on` is what the space
 * last said, `now` what the page shows. A flip shows at once, the logo going or coming
 * back with it, and is then saved; one that didn't save returns to what the space last
 * said, the switch and the logo together. Off, the whole corner goes with them. Only one write is ever on its way (`busy`): a
 * flip made meanwhile changes what shows and waits, and once that write has settled the
 * page saves what it shows if the space doesn't hold it already. So two quick flips can't
 * land out of order and leave the page saying one thing over a space that says the other. */
const MARK_SCRIPT = `<script>(function(){var b=document.getElementById("mk"),m=document.querySelector(".foot .mark"),f=document.querySelector(".foot");if(!b)return;
var on=b.defaultChecked,now=on,busy=0;function show(v){now=v;b.checked=v;if(m)m.hidden=!v;if(f)f.hidden=!v}show(on);
window.addEventListener("pageshow",function(){show(now)});
function save(){if(busy||now===on)return;var want=now;busy=1;
fetch("/_shroom/mark",{method:"POST",headers:{"content-type":"application/json"},body:JSON.stringify({on:want})})
.then(function(r){if(!r.ok)throw 0;on=want}).catch(function(){show(on)}).then(function(){busy=0;save()})}
b.onchange=function(){show(b.checked);save()}})();</script>`;

/** The thin bar under the line where the player will be: filled to the upload's
 * progress, breathing while bytes move, resting when they've stopped (N27, N40). */
/** @param {number} pct @param {boolean} rest */
function thinBar(pct, rest, id = "") {
  return `<span class="bar${rest ? "" : " go"}"><i${id ? ` id="${id}"` : ""} style="width:${Math.max(0, Math.min(100, Math.floor(pct)))}%"></i></span>`;
}

/** The recipient's on-its-way faces (N27, N40): the title stays, and the line sits where
 * the player will be, over the thin bar. The page refreshes itself. */
/** @param {string | null} title @param {string} line @param {Face} face @param {number} pct @param {boolean} stalled
 * @param {boolean} [unknown] no record at all: the stall clock runs from this tab's first look */
function onItsWayPage(title, line, face, pct, stalled, unknown = false) {
  const heading = title ? `<div class="head"><h1>${escapeText(title)}</h1></div>` : "";
  return shell(title || "shroom", `${heading}<div class="wait"><p class="line">${escapeText(line)}</p>${thinBar(pct, stalled)}</div>`, face, `<meta http-equiv="refresh" content="5">`, unknown ? stallClock() : "");
}

/** An id with no record has no progress to age (N40 “on its way, stalled”: “a mistyped link
 * lands here too, by design”): the clock runs from this tab's first look at the address —
 * kept across the page's own refreshes in the tab's session storage — and past
 * `STALL_SECONDS` the line becomes the stalled one and the bar rests. Without storage (a
 * blocked frame) the page stays on its way (DECISIONS B94). */
function stallClock() {
  return `<script>(function(){try{var s=sessionStorage,k="shroom-seen:"+location.pathname,t=Number(s.getItem(k));if(!t){s.setItem(k,String(Date.now()));return}
if(Date.now()-t<=${STALL_SECONDS * 1000})return;var p=document.querySelector("p.line")||document.querySelector(".line p"),b=document.querySelector(".bar");
if(p)p.textContent=${jsonForScript(JSON.stringify(COPY.stalled))};if(b)b.classList.remove("go")}catch(e){}})()</script>`;
}

/** The embed's unavailable face, when the record can't be read (B94). @param {unknown} e */
function unavailableEmbed(e) {
  return why(html(`<!doctype html><html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1">
<meta name="robots" content="noindex"><title>shroom</title><meta http-equiv="refresh" content="30"><style>${EMBED_TOKENS}${EMBED_STYLE}</style></head>
<body><div class="line"><p>${escapeText(COPY.unavailable)}</p></div></body></html>`, true), e);
}

/** @param {Face} face */
function notFoundPage(face) {
  return shell("shroom", `<p class="line">${escapeText(COPY.notFound)}</p>`, face);
}

/** @param {Face} face */
function unavailablePage(face) {
  return shell("shroom", `<p class="line">${escapeText(COPY.unavailable)}</p>`, face, `<meta http-equiv="refresh" content="30">`);
}

/** The player's names for its controls, handed to the page's script. */
const LABELS = { play: COPY.play, pause: COPY.pause, full: COPY.fullscreen, exit: COPY.exitFullscreen, speed: COPY.speed, mute: COPY.mute, unmute: COPY.unmute };

const ICON = {
  big: `<svg viewBox="0 0 16 16" aria-hidden="true"><path d="M5 3v10l8.5-5z"/></svg>`,
  play: `<svg class="i-play" viewBox="0 0 16 16" aria-hidden="true"><path d="M4.5 2.5v11l9-5.5z"/></svg>`,
  pause: `<svg class="i-pause" viewBox="0 0 16 16" aria-hidden="true"><path d="M4 2.5h2.8v11H4zM9.2 2.5H12v11H9.2z"/></svg>`,
  sound: `<svg class="i-on" viewBox="0 0 16 16" aria-hidden="true"><path d="M2 6.2h2.3L7.5 3.5v9L4.3 9.8H2z"/><path d="M10 6a3 3 0 0 1 0 4M11.8 4a5.6 5.6 0 0 1 0 8"/></svg>`,
  muted: `<svg class="i-off" viewBox="0 0 16 16" aria-hidden="true"><path d="M2 6.2h2.3L7.5 3.5v9L4.3 9.8H2z"/><path d="M10.4 6.2l3.6 3.6M14 6.2l-3.6 3.6"/></svg>`,
  full: `<svg class="i-enter" viewBox="0 0 16 16" aria-hidden="true"><path d="M2.5 6V3.5H5M11 3.5h2.5V6M13.5 10v2.5H11M5 12.5H2.5V10"/></svg>`,
  exitFull: `<svg class="i-exit" viewBox="0 0 16 16" aria-hidden="true"><path d="M5 2.5V5H2.5M13.5 5H11V2.5M11 13.5V11h2.5M2.5 11H5v2.5"/></svg>`,
};

/** A clock reading: 0:07, 4:32, 1:02:09 — the same as the page's script prints. The
 * position counts whole seconds down; a length rounds (a stream that reports 7.98 s for an
 * 8 s recording still reads 0:08, as the record said before the stream loaded). */
/** @param {number} seconds */
export function clock(seconds, length = false) {
  const n = Math.max(0, Number(seconds) || 0);
  const s = length ? Math.round(n) : Math.floor(n);
  const h = Math.floor(s / 3600), m = Math.floor((s % 3600) / 60), x = s % 60;
  const two = (/** @type {number} */ n) => String(n).padStart(2, "0");
  return h ? `${h}:${two(m)}:${two(x)}` : `${m}:${two(x)}`;
}

/** The speed button's words: "Playback speed, 1.5×". @param {number} rate */
const speedLabel = (rate) => `${COPY.speed}, ${rate}×`;

/** The player (Region 4: first frame, play, scrub bar, `0:00 / 4:32`, fullscreen — custom
 * controls, R6.2; the restyle adds speed and sound). Server-rendered, so the poster — the
 * recording's first frame — starts loading with the page, a big round play over it until
 * the first play (the picture takes that click). The controls are real buttons and a
 * range (a slider to assistive tech, its value read as the time), every one reachable by
 * keyboard: play first, then the seek bar drawn along the bar's top edge. `id="v"` stays
 * the video's (the heartbeat drives it). */
/** @param {string} id @param {any} meta @param {string} [base] the poster's origin, for the framable embed */
export function playerHtml(id, meta, base = "") {
  const m = meta?.media ?? {};
  const dur = Number(meta?.durationSec) > 0 ? Number(meta.durationSec) : 0;
  const poster = m.poster ? ` poster="${escapeHtml(`${base}/m/${id}/poster.jpg`)}"` : "";
  const ratio = m.width > 0 && m.height > 0 ? ` style="--ar:${Math.round(m.width)}/${Math.round(m.height)}"` : "";
  const time = `0:00${dur ? ` / ${clock(dur, true)}` : ""}`;
  return `<div class="player" data-dur="${dur}"${ratio}><video id="v" playsinline preload="metadata"${poster}></video>`
    + `<span class="big" aria-hidden="true">${ICON.big}</span>`
    + `<div class="ctl"><div class="row"><button type="button" class="pp" aria-label="${escapeHtml(COPY.play)}">${ICON.play}${ICON.pause}</button>`
    + `<input type="range" class="seek" min="0" max="${dur}" step="0.1" value="0" aria-label="${escapeHtml(COPY.seek)}" aria-valuetext="${time}">`
    + `<span class="time">${time}</span><span class="gap"></span>`
    + `<button type="button" class="sp" aria-label="${escapeHtml(speedLabel(1))}">1×</button>`
    + `<button type="button" class="mu" aria-label="${escapeHtml(COPY.mute)}">${ICON.sound}${ICON.muted}</button>`
    + `<button type="button" class="fs" aria-label="${escapeHtml(COPY.fullscreen)}">${ICON.full}${ICON.exitFull}</button></div></div></div>`;
}

/** The page's player script. `shroomPlay` attaches the stream: native HLS where the
 * browser has it (Safari, recent Chrome), the bucket's pinned hls.js otherwise
 * (backBufferLength keeps multi-hour tabs from growing without end). `shroomPlayer` wires
 * the custom controls: click or Space/K plays and pauses, the range and ←/→ (5 s),
 * Home/End scrub, M mutes, the speed button steps 1× → 1.25× → 1.5× → 2× and round, F or
 * double-click is fullscreen (the player itself, so its controls stay; a phone's own
 * fullscreen where elements can't go fullscreen). The seek bar shows what has played and
 * what is buffered ahead. The controls fade while playing and the pointer rests, never
 * while a control holds focus. */
export const PLAYER_JS = `function shroomPlay(v,s){if(/\\.mp4$/.test(s)||v.canPlayType("application/vnd.apple.mpegurl")){v.src=s;return}
var t=document.createElement("script");t.src="/_shroom/hls.js";t.onload=function(){var h=new Hls({backBufferLength:90,maxBufferLength:6,maxMaxBufferLength:6});
v.addEventListener("play",function(){h.config.maxBufferLength=30;h.config.maxMaxBufferLength=600},{once:true});h.loadSource(s);h.attachMedia(v)};document.head.appendChild(t)}
function shroomClock(s,l){s=Math.max(0,+s||0);s=l?Math.round(s):Math.floor(s);var h=Math.floor(s/3600),m=Math.floor(s%3600/60),x=s%60,two=function(n){return(n<10?"0":"")+n};return h?h+":"+two(m)+":"+two(x):m+":"+two(x)}
function shroomPlayer(p,s,L){var v=p.querySelector("video"),pp=p.querySelector(".pp"),sk=p.querySelector(".seek"),tm=p.querySelector(".time"),fs=p.querySelector(".fs"),sp=p.querySelector(".sp"),mu=p.querySelector(".mu"),known=+p.getAttribute("data-dur")||0,held=false,idle=0,R=[1,1.25,1.5,2];
function dur(){return isFinite(v.duration)&&v.duration>0?v.duration:known}
function ahead(t){var b=v.buffered,e=0;try{for(var i=0;i<b.length;i++)if(b.start(i)<=t+1&&b.end(i)>e)e=b.end(i)}catch(x){}return e}
function paint(){var d=dur(),t=held?+sk.value:v.ended&&d?d:v.currentTime||0,txt=shroomClock(t,v.ended)+(d?" / "+shroomClock(d,true):"");tm.textContent=txt;sk.max=String(d||0);if(!held)sk.value=String(t);sk.setAttribute("aria-valuetext",txt);
sk.style.setProperty("--p",String(d?Math.min(1,t/d):0));sk.style.setProperty("--b",String(d?Math.min(1,ahead(t)/d):0))}
function wake(){p.classList.remove("idle");clearTimeout(idle);if(!v.paused&&!v.ended)idle=setTimeout(function(){var a=document.activeElement;if(!(a&&a!==v&&p.contains(a)))p.classList.add("idle")},2500)}
function mode(){var on=!v.paused&&!v.ended;p.classList.toggle("playing",on);if(on)p.classList.add("started");pp.setAttribute("aria-label",on?L.pause:L.play);wake()}
function toggle(){if(v.paused||v.ended){var r=v.play();if(r&&r.catch)r.catch(function(){})}else v.pause()}
function to(t){var d=dur();if(!d)return;v.currentTime=Math.max(0,Math.min(d,t));paint()}
function rate(){var x=Math.round((v.playbackRate||1)*100)/100+"\\u00d7";sp.textContent=x;sp.setAttribute("aria-label",L.speed+", "+x)}
function faster(){var n=R[(R.indexOf(v.playbackRate)+1)%R.length];v.defaultPlaybackRate=n;v.playbackRate=n}
function sound(){p.classList.toggle("muted",v.muted);mu.setAttribute("aria-label",v.muted?L.unmute:L.mute)}
function hush(){v.muted=!v.muted}
function full(){return document.fullscreenElement||document.webkitFullscreenElement||null}
function fsToggle(){if(full()){var x=document.exitFullscreen||document.webkitExitFullscreen;if(x){var r=x.call(document);if(r&&r.catch)r.catch(function(){})}}
else if(p.requestFullscreen){p.requestFullscreen().catch(function(){})}else if(p.webkitRequestFullscreen){p.webkitRequestFullscreen()}else if(v.webkitEnterFullscreen){v.webkitEnterFullscreen()}}
function fsMode(){var on=full()===p;p.classList.toggle("full",on);fs.setAttribute("aria-label",on?L.exit:L.full)}
function key(e){if(e.metaKey||e.ctrlKey||e.altKey)return;var k=e.key,b=e.target&&e.target.tagName==="BUTTON";if((k===" "||k==="Enter")&&b)return;
if(k===" "||k==="k"||k==="K")toggle();else if(k==="ArrowLeft"||k==="ArrowRight")to((v.currentTime||0)+(k==="ArrowLeft"?-5:5));
else if(k==="Home")to(0);else if(k==="End")to(dur());else if(k==="f"||k==="F")fsToggle();else if(k==="m"||k==="M")hush();else return;e.preventDefault();wake()}
pp.addEventListener("click",toggle);v.addEventListener("click",toggle);v.addEventListener("dblclick",fsToggle);fs.addEventListener("click",fsToggle);sp.addEventListener("click",faster);mu.addEventListener("click",hush);
["play","pause","ended"].forEach(function(e){v.addEventListener(e,mode)});
["timeupdate","durationchange","loadedmetadata","seeked","emptied","progress"].forEach(function(e){v.addEventListener(e,paint)});
v.addEventListener("ratechange",rate);v.addEventListener("volumechange",sound);
sk.addEventListener("input",function(){held=true;to(+sk.value)});sk.addEventListener("change",function(){held=false;to(+sk.value)});
document.addEventListener("fullscreenchange",fsMode);document.addEventListener("webkitfullscreenchange",fsMode);
p.addEventListener("mousemove",wake);p.addEventListener("touchstart",wake,{passive:true});p.addEventListener("focusin",wake);p.addEventListener("keydown",key);
document.addEventListener("keydown",function(e){if(e.target===document.body)key(e)});
shroomPlay(v,s);paint();rate();sound()}`;

/** Copy to the clipboard, saying whether it happened: the Clipboard API, else the older
 * copy command; when neither works, the visible text (if any) is selected so ⌘C does
 * it — and the button keeps its label, since nothing was copied (N1). */
export const COPY_JS = `function shroomCopy(t,el){function old(){var a=document.createElement("textarea"),r=false;a.value=t;a.setAttribute("readonly","");a.style.cssText="position:fixed;top:0;left:0;opacity:0";document.body.appendChild(a);a.select();
try{r=document.execCommand("copy")===true}catch(e){r=false}a.remove();if(!r&&el){var g=document.createRange(),s=window.getSelection();g.selectNodeContents(el);s.removeAllRanges();s.addRange(g)}return r}
return navigator.clipboard&&navigator.clipboard.writeText?navigator.clipboard.writeText(t).then(function(){return true},old):Promise.resolve(old())}`;

/** Copy thumbnail's picture (N46, B164), drawn in the owner's browser after each title save:
 * the video's first frame (a still, 1200 wide) and then, once the teaser exists, a 3 s loop
 * of its start (640 wide, 10 fps) — the title on a soft band at the foot, the length in a
 * corner, a play mark in the middle; untitled, no band. A GIF needs no library: a palette cut from
 * the frames' colours (median cut on 5 bits a channel, weighted so a small coloured text
 * keeps its colour), an ordered dither so the band's shade doesn't step, unchanged pixels
 * left out of each frame, LZW (Bagrat on 0.1.15: "gif quality is pretty low"). Each picture is stored for the title it was drawn for; the
 * copy wraps the newest in the link. */
export const THUMB_JS = `function shroomGif(frames,w,h,delay){var n=w*h,hist=new Uint32Array(32768),i,f,k,d;
for(f=0;f<frames.length;f+=Math.max(1,Math.floor(frames.length/6))){d=frames[f];for(k=0;k<n*4;k+=4)hist[((d[k]>>3)<<10)|((d[k+1]>>3)<<5)|(d[k+2]>>3)]++}
var cols=[];for(i=0;i<32768;i++)if(hist[i])cols.push(i);
function ch(c,a){return a===0?c>>10:a===1?(c>>5)&31:c&31}function wt(c){return Math.sqrt(hist[c])}
var boxes=[cols];while(boxes.length<255){var bi=-1,best=0,axis=0;for(var b=0;b<boxes.length;b++){var bx=boxes[b];if(bx.length<2)continue;for(var a=0;a<3;a++){var lo=31,hi=0,sw=0;for(k=0;k<bx.length;k++){var v=ch(bx[k],a);if(v<lo)lo=v;if(v>hi)hi=v;sw+=wt(bx[k])}var sc=(hi-lo)*Math.sqrt(sw);if(sc>best){best=sc;bi=b;axis=a}}}
if(bi<0)break;var sb=boxes[bi].sort(function(x,y){return ch(x,axis)-ch(y,axis)}),tot=0,acc=0,cut=0;for(k=0;k<sb.length;k++)tot+=wt(sb[k]);for(;cut<sb.length-1;cut++){acc+=wt(sb[cut]);if(acc>=tot/2)break}boxes.splice(bi,1,sb.slice(0,cut+1),sb.slice(cut+1))}
var pal=new Uint8Array(768),np=boxes.length,map=new Uint8Array(32768);
for(i=0;i<np;i++){var r=0,g=0,bl=0,sum=0;for(k=0;k<boxes[i].length;k++){var c=boxes[i][k],hw=hist[c];r+=((c>>10)*8+4)*hw;g+=(((c>>5)&31)*8+4)*hw;bl+=((c&31)*8+4)*hw;sum+=hw}pal[i*3]=r/sum;pal[i*3+1]=g/sum;pal[i*3+2]=bl/sum}
for(i=0;i<32768;i++){var cr=(i>>10)*8+4,cg=((i>>5)&31)*8+4,cb=(i&31)*8+4,bj=0,bd=1e9;for(var j=0;j<np;j++){var dr=pal[j*3]-cr,dg=pal[j*3+1]-cg,db=pal[j*3+2]-cb,e=dr*dr*3+dg*dg*4+db*db*2;if(e<bd){bd=e;bj=j;if(!e)break}}map[i]=bj}
var bayer=[0,8,2,10,12,4,14,6,3,11,1,9,15,7,13,5],out=[],prev=null;function u16(v){out.push(v&255,v>>8)}function str(s){for(var q=0;q<s.length;q++)out.push(s.charCodeAt(q))}function cl(v){return v<0?0:v>255?255:v}
str("GIF89a");u16(w);u16(h);out.push(0xf7,0,0);for(i=0;i<768;i++)out.push(pal[i]);
out.push(0x21,0xff,11);str("NETSCAPE2.0");out.push(3,1,0,0,0);
for(f=0;f<frames.length;f++){d=frames[f];var idx=new Uint8Array(n);for(var y=0;y<h;y++)for(var x=0;x<w;x++){var kk=y*w+x,o=kk*4,t=(bayer[(y&3)*4+(x&3)]/16-.5)*12;
idx[kk]=map[((cl(d[o]+t)>>3)<<10)|((cl(d[o+1]+t)>>3)<<5)|(cl(d[o+2]+t)>>3)]}
if(prev){for(k=0;k<n;k++){if(idx[k]===prev[k])idx[k]=255;else prev[k]=idx[k]}}else prev=idx.slice();
out.push(0x21,0xf9,4,f?5:4,delay&255,delay>>8,255,0,0x2c);u16(0);u16(0);u16(w);u16(h);out.push(0,8);
shroomLzw(idx,out)}
out.push(0x3b);return new Uint8Array(out)}
function shroomLzw(idx,out){var clear=256,eoi=257,size=9,next=258,dict=new Map(),acc=0,bits=0,block=[];
function emit(c){acc|=c<<bits;bits+=size;while(bits>=8){block.push(acc&255);acc>>>=8;bits-=8;if(block.length===255){out.push(255);for(var q=0;q<255;q++)out.push(block[q]);block=[]}}}
emit(clear);var w=idx[0];for(var i=1;i<idx.length;i++){var k=idx[i],key=w*256+k,hit=dict.get(key);if(hit!==undefined){w=hit;continue}
emit(w);if(next<4096){dict.set(key,next++);if(next>(1<<size)&&size<12)size++}else{emit(clear);dict.clear();size=9;next=258}w=k}
emit(w);emit(eoi);if(bits>0)block.push(acc&255);if(block.length){out.push(block.length);for(var q=0;q<block.length;q++)out.push(block[q])}out.push(0)}
function shroomThumbs(c,cb){var gen=0,making=null,guard=0,F="-apple-system,BlinkMacSystemFont,system-ui,sans-serif";
function size(W){return[W,Math.round(W*c.rh/c.rw/2)*2]}
function draw(x,W,H,src,title){x.drawImage(src,0,0,W,H);
if(title){var g=x.createLinearGradient(0,H,0,H*.58);g.addColorStop(0,"rgba(31,34,40,.92)");g.addColorStop(1,"rgba(31,34,40,0)");x.fillStyle=g;x.fillRect(0,H*.58,W,H*.42)}
var r=W*.075,cx=W/2,cy=H/2;x.beginPath();x.arc(cx,cy,r,0,7);x.fillStyle="rgba(46,52,64,.72)";x.fill();x.lineWidth=Math.max(2,W/300);x.strokeStyle="rgba(236,239,244,.9)";x.stroke();
x.beginPath();x.moveTo(cx-r*.3,cy-r*.42);x.lineTo(cx-r*.3,cy+r*.42);x.lineTo(cx+r*.45,cy);x.closePath();x.fillStyle="#eceff4";x.fill();
var f=Math.round(W*.026),pad=W*.012,bx=W,bh=f*1.6;x.font="500 "+f+"px "+F;x.textBaseline="middle";
if(c.len){var bw=x.measureText(c.len).width+pad*2,by=H-W*.02-bh;bx=W-W*.02-bw;x.fillStyle="rgba(46,52,64,.85)";x.beginPath();if(x.roundRect)x.roundRect(bx,by,bw,bh,f*.3);else x.rect(bx,by,bw,bh);x.fill();x.fillStyle="#eceff4";x.fillText(c.len,bx+pad,by+bh/2)}
if(title){var tf=Math.round(W*.04),max=bx-W*.06,t=title;x.font="600 "+tf+"px "+F;x.textBaseline="alphabetic";if(x.measureText(t).width>max){while(t&&x.measureText(t+"…").width>max)t=t.slice(0,-1);t+="…"}x.fillStyle="#eceff4";x.fillText(t,W*.03,H-W*.035)}}
function canvas(W,H){var k=document.createElement("canvas");k.width=W;k.height=H;return k}
function still(title){var img=new Image();img.src="/m/"+c.id+"/poster.jpg";return img.decode().then(function(){var s=size(1200),k=canvas(s[0],s[1]);draw(k.getContext("2d"),s[0],s[1],img,title);
return jpeg(k,.85).then(function(b){return b&&b.size>c.caps.jpgMax?jpeg(k,.7):b})})}
function jpeg(k,q){return new Promise(function(ok){k.toBlob(ok,"image/jpeg",q)})}
function loop(title){var v=document.createElement("video");v.muted=true;v.playsInline=true;v.preload="auto";v.src="/m/"+c.id+"/teaser.mp4";
return new Promise(function(ok,no){v.addEventListener("loadeddata",ok,{once:true});v.addEventListener("error",no,{once:true})}).then(function(){
var s=size(c.caps.gifWidth),k=canvas(s[0],s[1]),x=k.getContext("2d",{willReadFrequently:true}),n=Math.min(30,Math.floor((v.duration||3)*10)),frames=[],i=0;
function step(){if(i>=n)return Promise.resolve();return new Promise(function(ok){v.addEventListener("seeked",ok,{once:true});v.currentTime=i/10}).then(function(){x.clearRect(0,0,s[0],s[1]);draw(x,s[0],s[1],v,title);
if(!i&&!x.getImageData(Math.round(s[0]*.05),Math.round(s[1]*.05),1,1).data[3])throw new Error("no frames");frames.push(x.getImageData(0,0,s[0],s[1]).data);i++;return step()})}
return step().then(function(){var g=shroomGif(frames,s[0],s[1],10);if(g.length>c.caps.gifMax){g=shroomGif(frames.filter(function(_,j){return j%2===0}),s[0],s[1],20)}v.removeAttribute("src");return g.length>c.caps.gifMax?null:new Blob([g],{type:"image/gif"})})})}
function store(b,title){if(!b)return null;return fetch("/_shroom/thumb/"+c.id,{method:"POST",headers:{"content-type":b.type,"x-shroom-title":encodeURIComponent(title)},body:b})
.then(function(r){return r.ok?r.json():null}).then(function(j){return j&&j.url},function(){return null})}
function wake(my){if(my===gen){clearTimeout(guard);cb(true)}}
function make(){var my=++gen,title=c.titled?c.title:"";
var first=c.thumb?Promise.resolve(c.thumb):still(title).then(function(b){return store(b,title)}).then(function(u){if(my===gen&&u&&!c.thumb)c.thumb=u;return u},function(){return null});
making=first;first.then(function(){if(my!==gen)return;if(c.gif)return wake(my);if(!c.teaser){if(c.drained)wake(my);return}
clearTimeout(guard);guard=setTimeout(function(){wake(my)},20000);
return loop(title).then(function(b){return store(b,title)}).then(function(u){if(my===gen&&u){c.thumb=u;c.gif=true}wake(my)})}).catch(function(){wake(my)});
return first}
function html(u){var w=600,h=Math.round(600*c.rh/c.rw),t=c.titled?c.title:"",a=document.createElement("a"),i=document.createElement("img");a.href=location.origin+"/"+c.id;
i.src=location.origin+u;i.width=w;i.height=h;i.alt=t;i.style.maxWidth="100%";a.appendChild(i);return a.outerHTML}
if(!c.store)make=function(){return Promise.resolve(c.thumb)};cb=cb||function(){};
return{length:function(l){if(l)c.len=l},retitle:function(t){c.title=t;c.titled=true;c.thumb=null;c.gif=false;if(c.live){cb(false);make()}},
drained:function(){if(c.drained)return;c.drained=true;var my=gen;if(!c.teaser&&making)making.then(function(){if(!c.teaser)wake(my)})},
live:function(){c.live=true;if(!c.thumb||!c.gif)make()},
teaser:function(){if(c.teaser)return;c.teaser=true;if(c.live&&!c.gif)make()},
copy:function(){var link=location.origin+"/"+c.id;return shroomCopyRich(c.thumb?html(c.thumb):(making||make()).then(function(u){return html(u||"/m/"+c.id+"/poster.jpg")}),link)}}}
function shroomCopyRich(h,t){var hp=Promise.resolve(h);
function old(x){var ok=false;function on(e){e.preventDefault();e.clipboardData.setData("text/html",x);e.clipboardData.setData("text/plain",t);ok=true}document.addEventListener("copy",on);try{document.execCommand("copy")}catch(e){}document.removeEventListener("copy",on);return ok}
if(navigator.clipboard&&navigator.clipboard.write&&window.ClipboardItem){try{return navigator.clipboard.write([new ClipboardItem({"text/html":hp.then(function(x){return new Blob([x],{type:"text/html"})}),"text/plain":new Blob([t],{type:"text/plain"})})]).then(function(){return true},function(){return hp.then(old)})}catch(e){}}
return hp.then(old)}`;

/** @param {string} id @param {any} meta @param {Face} face @param {string} src what the player plays (B177) */
function livePage(id, meta, face, src) {
  const title = meta.title || "";
  // Download is a quiet icon beside the title: its word names it.
  const download = meta.media.mp4 ? `<span class="dl" data-tip="${COPY.download}"><a href="/d/${id}">${COPY.download}</a></span>` : "";
  // Copy link and Copy thumbnail for viewers too (N46): a viewer stores nothing, so Copy
  // thumbnail shows only once the owner's page has made a picture for the title.
  const thumbs = thumbsOf(id, meta, true, false);
  const cfg = JSON.stringify({ copied: COPY.copied, copy: COPY.copyLink, copyThumb: COPY.copyThumbnail, thumbs });
  const player = `${playerHtml(id, meta)}
<script>${PLAYER_JS};shroomPlayer(document.querySelector(".player"),${JSON.stringify(src)},${jsonForScript(JSON.stringify(LABELS))})
${COPY_JS}
${THUMB_JS}
(function(){var c=${jsonForScript(cfg)},th=shroomThumbs(c.thumbs);
function copied(el,label){return function(ok){if(!ok)return;el.textContent=c.copied;el.classList.add("done");setTimeout(function(){el.textContent=label;el.classList.remove("done")},1500)}}
var b=document.getElementById("c");b.onclick=function(){shroomCopy(location.origin+"/"+c.thumbs.id,null).then(copied(b,c.copy))};
var tb=document.getElementById("th");tb.onclick=function(){th.copy().then(copied(tb,c.copyThumb))}})()</script>`;
  return shell(title || "shroom", `<div class="head"><h1>${escapeText(title)}</h1>${copyButtons(!!thumbs.thumb)}${download}</div>${player}`, face);
}

/** Copy link and Copy thumbnail, each named by its words and with them as its tooltip (the
 * icons alone could read as something else — a box with a play mark, as "play"). Copy
 * thumbnail waits, greyed and saying so, until its picture is ready (N46). */
/** @param {boolean} shown @param {boolean} [waiting] */
const copyButtons = (shown, waiting = false) => `<button class="copy" id="c" data-tip="${COPY.copyLink}">${COPY.copyLink}</button>`
  + `<button class="copy thumb" id="th" data-tip="${waiting ? COPY.preparingThumbnail : COPY.copyThumbnail}"${waiting ? ' aria-disabled="true"' : ""}${shown ? "" : " hidden"}>${COPY.copyThumbnail}</button>`;

/** The newest picture made for the title a video has now (N46), as its address, or null. */
/** @param {any} meta @returns {string | null} */
function newestThumb(meta) {
  const title = meta.titled ? meta.title : null;
  const t = meta.thumb && typeof meta.thumb === "object" && meta.thumb.title === title ? meta.thumb : null;
  const file = t && (typeof t.gif === "string" ? t.gif : typeof t.jpg === "string" ? t.jpg : null);
  return typeof file === "string" && ID.test(String(meta.id)) && /^thumb-[0-9a-f]{16}\.(gif|jpg)$/.test(file) ? `/m/${meta.id}/${file}` : null;
}

/** What a page's Copy thumbnail starts from (N46): the picture made for the title the video
 * has now — the loop over the still — and what drawing the next one needs. */
/** @param {string} id @param {any} meta @param {boolean} live */
function thumbsOf(id, meta, live, store = true) {
  const thumb = newestThumb({ ...meta, id });
  return { id, live, store, title: meta.titled ? meta.title : "", titled: !!meta.titled, thumb, gif: !!thumb?.endsWith(".gif"),
    teaser: !!meta.media?.teaser, len: Number(meta.durationSec) > 0 ? clock(meta.durationSec, true) : "",
    rw: meta.media?.width || 16, rh: meta.media?.height || 9, caps: THUMB };
}

/** The owner page (N26, N40): the title field is the only chrome, Copy link and Download
 * quiet icons beside it; the upload line under the player, gone once every file is up;
 * the switch bottom-right for owner-class browsers. It never reloads itself — the title
 * field may hold typing — it polls and swaps in place: the player at the go-live,
 * Download when the single-file copy lands, the removed or unavailable line if the video
 * goes (B43). Until the go-live the stage is the player's empty frame — the upload line
 * speaks; the recipient's "refreshes itself" line is theirs (B43). */
/** @param {string} id @param {any} meta @param {number} now @param {Face} face @param {string} waiting the stalled line
 * @param {boolean} [blocked] the space can't take uploads: the waiting line from the start
 * @param {boolean} [pending] no record yet (its claim is still on its way) */
function ownerPage(id, meta, now, face, waiting, blocked = false, pending = false, src = `/m/${id}/master.m3u8`) {
  const live = meta.state === "live";
  const placeholder = meta.titled ? untitled(meta.createdAt ?? new Date(now).toISOString()) : meta.title || untitled(meta.createdAt);
  const value = meta.titled ? meta.title : "";
  const st = status(meta, now);
  const stalled = blocked || !!st.stalled;
  const drainedNow = live && st.drained;
  // Drained, the upload line has nothing left to say and goes (no “Fully live ✓” on the page);
  // an open page that saw it drain says “Ready ✓” for a few seconds first (B142).
  const line = drainedNow ? "" : stalled ? waiting : uploadingLine(st.pct ?? 0);
  const field = `<input class="title" id="t" maxlength="${TITLE_MAX}" aria-label="Title" value="${escapeHtml(value)}" placeholder="${escapeHtml(placeholder)}"${meta.titled ? "" : " autofocus"}>`;
  const frame = meta.media?.poster ? `<img src="/m/${id}/poster.jpg" alt="">` : "";
  const stage = live ? playerHtml(id, meta) : `<div class="wait" id="w">${frame}</div>`;
  const download = meta.media?.mp4 ? `<a href="/d/${id}">${COPY.download}</a>` : "";
  // Copy thumbnail waits for the loop made for this title — or, when none can be made (the
  // upload finished with no teaser), for the still (N46).
  const thumbs = { ...thumbsOf(id, meta, live), drained: drainedNow };
  const ready = thumbs.gif || (drainedNow && !meta.media?.teaser && !!thumbs.thumb);
  // Copy link and Download are quiet icons beside the title; their words name them.
  const head = `<div class="head">${field}${copyButtons(live, !ready)}<span id="dl" data-tip="${COPY.download}">${download}</span></div>`;
  const upload = `<div class="upl" id="ul"${drainedNow ? " hidden" : ""}><span class="up" id="u">${escapeText(line)}</span>${drainedNow ? "" : thinBar(st.pct ?? 0, stalled, "b")}</div>`;
  const [upA, upB] = uploadingLine(0).split("0%");
  const cfg = JSON.stringify({
    id, live, drained: drainedNow, pending, mp4: !!meta.media?.mp4, saved: meta.titled ? meta.title : "", copied: COPY.copied, copy: COPY.copyLink, waiting,
    copyThumb: COPY.copyThumbnail, preparing: COPY.preparingThumbnail, thumbs, dl: COPY.download, ready: COPY.ready, notFound: COPY.notFound, unavailable: COPY.unavailable, upA, upB, labels: LABELS, player: live ? "" : playerHtml(id, { ...meta, media: { ...meta.media, poster: null } }),
  });
  const script = `<script>${PLAYER_JS}
${COPY_JS}
${THUMB_JS}
(function(){var c=${jsonForScript(cfg)},t=document.getElementById("t"),saved=c.saved,src=${jsonForScript(JSON.stringify(src))},tb=document.getElementById("th");
var th=shroomThumbs(c.thumbs,function(ok){if(ok)tb.removeAttribute("aria-disabled");else tb.setAttribute("aria-disabled","true");tb.setAttribute("data-tip",ok?c.copyThumb:c.preparing)});
if(c.live){shroomPlayer(document.querySelector(".player"),src,c.labels);th.live()}
function send(v){c.pend=null;fetch("/_shroom/title/"+c.id,{method:"POST",headers:{"content-type":"application/json"},body:JSON.stringify({title:v})})
.then(function(r){if(r.status===404)c.pend=v;return r.ok?r.json():null}).then(function(j){if(j){saved=j.title;if(document.activeElement!==t)t.value=saved;document.title=saved;th.retitle(saved)}})}
function save(){var v=t.value.replace(/\\s+/g," ").trim();if(!v){t.value=saved;return}if(v===saved)return;send(v)}
t.addEventListener("keydown",function(e){if(e.key==="Enter"){e.preventDefault();t.blur()}});t.addEventListener("blur",save);
function copied(el,label){return function(ok){if(!ok)return;el.textContent=c.copied;el.classList.add("done");setTimeout(function(){el.textContent=label;el.classList.remove("done")},1500)}}
var b=document.getElementById("c");b.onclick=function(){shroomCopy(location.origin+"/"+c.id,null).then(copied(b,c.copy))};
tb.onclick=function(){if(tb.getAttribute("aria-disabled"))return;th.copy().then(copied(tb,c.copyThumb))};
var ul=document.getElementById("ul"),up=document.getElementById("u"),bar=document.getElementById("b"),dl=document.getElementById("dl"),gap=2000;
function next(ms){setTimeout(poll,ms)}
function showBar(on,moving){if(bar){bar.parentNode.style.display=on?"":"none";bar.parentNode.className="bar"+(moving?" go":"")}}
function say(x){var w=document.getElementById("w");if(!w||(!x&&!c.said))return;c.said=!!x;w.textContent="";if(x){var p=document.createElement("p");p.className="line";p.textContent=x;w.appendChild(p)}}
function poll(){fetch("/_shroom/status/"+c.id,{cache:"no-store"}).then(function(r){return r.json()}).then(function(s){
if(s.state==="removed"){var m=document.querySelector("main"),p=document.createElement("p");m.textContent="";p.className="line";p.textContent=c.notFound;m.appendChild(p);document.title="shroom";return}
if(c.pend&&s.state!=="waiting")send(c.pend);
if(s.state==="live"){if(s.dur)th.length(shroomClock(s.dur,true));if(s.teaser)th.teaser();if(!c.live){c.live=true;tb.hidden=false;th.live();var w=document.getElementById("w"),h=document.createElement("div");h.innerHTML=c.player;var pl=h.firstChild;
if(s.poster)pl.querySelector("video").poster="/m/"+c.id+"/poster.jpg";w.replaceWith(pl);shroomPlayer(pl,src,c.labels)}
if(s.drained)th.drained();
if(s.drained){if(bar&&!c.done){c.done=true;up.textContent=c.ready;ul.classList.add("done");showBar(true,false);bar.style.width="100%";setTimeout(function(){ul.classList.add("fade");setTimeout(function(){ul.hidden=true;up.textContent="";if(bar){bar.parentNode.remove();bar=null}},400)},3000)}else if(!c.done){ul.hidden=true;up.textContent="";if(bar){bar.parentNode.remove();bar=null}}}else{ul.hidden=false;up.textContent=s.stalled?c.waiting:c.upA+s.pct+"%"+c.upB;showBar(true,!s.stalled);if(bar)bar.style.width=s.pct+"%"}
if(s.mp4){if(!dl.firstChild){var a=document.createElement("a");a.href="/d/"+c.id;a.textContent=c.dl;dl.appendChild(a)}if(s.drained)return}
if(!s.drained)return next(2000);gap=Math.min(gap*2,30000);return next(gap)}
if(s.state==="unavailable"){if(!c.live){say(c.unavailable);up.textContent="";showBar(false,false)}return next(30000)}
if(s.state==="uploading"){say("");ul.hidden=false;up.textContent=s.stalled?c.waiting:c.upA+s.pct+"%"+c.upB;showBar(true,!s.stalled);if(bar)bar.style.width=s.pct+"%"}
next(2000)}).catch(function(){next(5000)})}
if(!c.live||!c.mp4||!c.drained)next(2000)})();</script>`;
  return shell(meta.title || "shroom", `${head}${stage}${upload}`, face, "", script);
}

/** A JSON island safe inside <script>: <, >, & and the two line separators escaped. */
/** @param {string} s */
export function jsonForScript(s) {
  return s.replace(/[<>&\u2028\u2029]/g, (c) => `\\u${c.charCodeAt(0).toString(16).padStart(4, "0")}`);
}

// ════════════════════════════════════════════════════════════════════════════════════
// Region 6 — the space's members (WORKER-INTERFACE §2–§4)
// ════════════════════════════════════════════════════════════════════════════════════

const API = "/_shroom/api/";
const ROSTER = ".shroom/registry/roster.json";
const LEDGER = ".shroom/registry/recs/";
const RATE = ".shroom/registry/rate/";
const KEYS = ".shroom/keys/";
const HEALTH = ".shroom/health.json";
/** Deletion records (DECISIONS B125): `.shroom/gone/<id>`, written only here or by a token
 * Mac — never reachable by a recording credential, so a video's own upload credential
 * (which may write its meta for days, N32) can't bring a deleted video back. */
const GONE = ".shroom/gone/";
const M1 = "m1";
const DEVICE_ID = /^d_[A-Za-z0-9_-]{16}$/;
const MEMBER_ID = /^m_[A-Za-z0-9_-]{16}$/;
const INVITE_ID = /^i_[A-Za-z0-9_-]{16}$/;
const HEX64 = /^[0-9a-f]{64}$/;
const HOUR_MS = 3600 * 1000;
const DAY_MS = 24 * HOUR_MS;
/** Invite lifetimes, set by the server's clock (SM:1328, 1387). */
const INVITE_SECONDS = { member: 604800, own_mac: 3600 };
/** Credential lifetimes (§3.4): R2 sets no ceiling of its own (B35 P6), so these are it. */
const TTL = { upload: 604800, manage: 900 };
/** A one-time page key's life (`shroom-cloud::owner::KEY_TTL_SECS`). */
const KEY_SECONDS = 600;
/** New recordings one device may open in a clock hour (H4; a scaffold flag). */
const UPLOAD_BUDGET = 20;
/** One-time page keys a device may ask for in an hour (B125): opening one's own page is a
 * click each, so this is room to spare — and a ceiling on what a member can make the
 * owner's bucket hold until the token Mac's sweep reaps the unspent ones. */
const KEY_BUDGET = 60;
/** The token-holder flag: a parent proof within this long (H1). */
const TOKEN_FLAG_MS = 30 * DAY_MS;
/** Enroll re-stamps proofAt at most this often (C5). */
const RESTAMP_MS = DAY_MS;
const CLOCK_SKEW_SECONDS = 300;
const CAS_TRIES = 5;
const BODY_MAX = 16 * 1024;
const LIBRARY_BODY_MAX = 128 * 1024;
/** Record bodies one Library call may fetch (P8: far inside the per-invocation ceiling). */
const LIBRARY_BODIES = 40;
const CAPS = { pending: 20, invites: 500, members: 200, ownerMacs: 20, label: 64, name: 64, inviter: 40 };
const JSON_TYPE = { contentType: "application/json" };

/** A classed refusal: `{"shroom":1,"error":<class>, …extra}` with its status. */
class Refusal {
  /** @param {number} status @param {string} error @param {Record<string, unknown>} [extra] */
  constructor(status, error, extra = {}) {
    this.status = status;
    this.error = error;
    this.extra = extra;
  }
}
const bad = () => new Refusal(400, "bad-request");
/** @param {string} reason */
const space = (reason) => new Refusal(503, "space", { reason });

/** @param {number} ms */
const iso = (ms) => new Date(ms).toISOString();
/** An own property of a roster table, never one of Object.prototype's. */
/** @param {any} table @param {string} key */
const own = (table, key) => (table && Object.prototype.hasOwnProperty.call(table, key) ? table[key] : undefined);

/** A shroom-shaped answer (§4's shape rule): JSON carrying "shroom":1. */
/** @param {Record<string, unknown>} body */
function answer(body, status = 200) {
  return json({ shroom: 1, ...body }, status);
}
function noContent() {
  return new Response(null, { status: 204, headers: { "x-robots-tag": "noindex", "cache-control": "no-store" } });
}

/** @param {Request} request @param {Env} env @param {URL} url @param {number} now */
async function api(request, env, url, now) {
  try {
    // Cross-site (§3.8): the app sends no Origin; a browser always does. Nothing is read.
    if (request.headers.has("origin")) throw new Refusal(403, "origin");
    return await route(request, env, url, now);
  } catch (e) {
    if (e instanceof Refusal) return answer({ error: e.error, ...e.extra }, e.status);
    // A binding error: never its message (it could name a key), just the class.
    return answer({ error: "space", reason: "storage" }, 503);
  }
}

/** @param {Request} request @param {Env} env @param {URL} url @param {number} now */
async function route(request, env, url, now) {
  const path = url.pathname.slice(API.length);
  const method = request.method;
  /** @param {string} want */
  const only = (want) => { if (method !== want) throw new Refusal(405, "method"); };
  switch (path) {
    case "peek": only("POST"); return peek(env, await body(request), now);
    case "join": only("POST"); return join(env, url, await body(request), now);
    case "enroll": only("POST"); return enroll(request, env, url, now);
    case "me": only("GET"); return me(request, env, url, now);
    case "mint": only("POST"); return mint(request, env, await body(request), now);
    case "key": only("POST"); return key(request, env, await body(request), now);
    case "library": only("POST"); return library(request, env, await body(request, LIBRARY_BODY_MAX));
    case "members": only("GET"); return members(request, env, now);
    case "invites": only("POST"); return invites(request, env, await body(request), now);
    case "mark": only("POST"); return mark(request, env, await body(request), now);
    case "health": only("POST"); return health(request, env, await body(request), now);
    case "gone": only("POST"); return gone(request, env, await body(request), now);
  }
  const target = path.match(/^(invites|members|devices)\/([^/]+)$/);
  if (target) {
    only("DELETE");
    if (target[1] === "invites") return cancelInvite(request, env, target[2], now);
    if (target[1] === "members") return removeMember(request, env, target[2], now);
    return removeDevice(request, env, target[2], now);
  }
  throw new Refusal(404, "no-route");
}

/** A JSON object body, at most `max` bytes (§3.8), or a classed refusal. */
/** @param {Request} request @returns {Promise<any>} */
async function body(request, max = BODY_MAX) {
  return (await rawBody(request, max)).value;
}
/** @param {Request} request @returns {Promise<{ value: any, bytes: Uint8Array<ArrayBuffer> }>} */
async function rawBody(request, max = BODY_MAX) {
  const bytes = await readCapped(request, max);
  /** @type {any} */
  let value;
  try { value = JSON.parse(new TextDecoder().decode(bytes)); } catch { throw bad(); }
  if (!value || typeof value !== "object" || Array.isArray(value)) throw bad();
  return { value, bytes };
}

/** The body's bytes, never more than `max` held: a declared length past the cap is
 * refused unread, and an undeclared one is cut off at the first chunk that crosses it. */
/** @param {Request} request @param {number} max @returns {Promise<Uint8Array<ArrayBuffer>>} */
async function readCapped(request, max) {
  if (Number(request.headers.get("content-length") ?? 0) > max) throw new Refusal(413, "too-large");
  if (!request.body) return new Uint8Array(0);
  const reader = request.body.getReader();
  /** @type {Uint8Array[]} */
  const chunks = [];
  let size = 0;
  for (;;) {
    const { done, value } = await reader.read();
    if (done) break;
    size += value.byteLength;
    if (size > max) {
      reader.cancel().catch(() => {});
      throw new Refusal(413, "too-large");
    }
    chunks.push(value);
  }
  const bytes = new Uint8Array(size);
  let at = 0;
  for (const c of chunks) { bytes.set(c, at); at += c.byteLength; }
  return bytes;
}

// ───────── the registry: one roster, every change one CAS (§2.2) ─────────

/** One fresh read — never cached across requests. Absent → `unwired`; present but not a
 * roster → `broken`, left exactly as found (the Worker never creates or repairs it). */
/** @param {Env} env @returns {Promise<{ roster: any, etag: string }>} */
async function readRoster(env) {
  /** @type {R2ObjectBody | null} */
  let object;
  /** @type {string} */
  let text;
  try {
    object = await env.BUCKET.get(ROSTER);
    if (!object) throw space("unwired");
    text = await object.text();
  } catch (e) {
    throw e instanceof Refusal ? e : space("storage");
  }
  /** @type {any} */
  let roster;
  try { roster = JSON.parse(text); } catch { throw space("broken"); }
  const table = (/** @type {unknown} */ t) => !!t && typeof t === "object" && !Array.isArray(t);
  if (!table(roster) || !table(roster.members) || !table(roster.devices) || !table(roster.invites)) throw space("broken");
  return { roster, etag: object.etag };
}

/** A conditional put; a refused precondition is `false` whether the binding answers null
 * or throws, and so is R2 refusing a fast second write to the key (B35 P1, C5). */
/** @param {Env} env @param {string} key @param {string} value @param {R2Conditional} onlyIf @param {Record<string, string>} [customMetadata] */
async function casPut(env, key, value, onlyIf, customMetadata) {
  try {
    const options = customMetadata ? { onlyIf, httpMetadata: JSON_TYPE, customMetadata } : { onlyIf, httpMetadata: JSON_TYPE };
    return (await env.BUCKET.put(key, value, options)) !== null;
  } catch {
    return false;
  }
}

/** Applies `fn` to a fresh roster and writes it back on the ETag it read, ≤ 5 tries with
 * jitter, then 503 busy. `fn` returns `{ write, result }` or throws a Refusal (nothing is
 * written). Pruning runs inside every change (U7). `first` reuses a read the request
 * already made (its auth) for the first try. */
/** @template T @param {Env} env @param {number} now @param {(roster: any) => { write?: boolean, result: T }} fn @param {{ roster: any, etag: string }} [first] @returns {Promise<T>} */
async function mutate(env, now, fn, first) {
  for (let attempt = 0; attempt < CAS_TRIES; attempt++) {
    if (attempt > 0) await new Promise((r) => setTimeout(r, 50 + Math.random() * 350));
    const read = attempt === 0 && first ? { roster: structuredClone(first.roster), etag: first.etag } : await readRoster(env);
    prune(read.roster, now);
    const out = fn(read.roster);
    if (!out.write) return out.result;
    if (await casPut(env, ROSTER, JSON.stringify(read.roster), { etagMatches: read.etag })) return out.result;
  }
  throw new Refusal(503, "busy");
}

/** Drops what nothing on screen still needs (§2.3, U7): ended Macs after 180 days; used
 * invites with the Mac they made; cancelled invites and expired own-Mac links after 30
 * days. Pending member invites and members are never pruned. */
/** @param {any} r @param {number} now */
function prune(r, now) {
  const past = (/** @type {unknown} */ at, /** @type {number} */ ms) => typeof at === "string" && Date.parse(at) + ms < now;
  for (const [id, d] of Object.entries(r.devices)) {
    if (d && d.state !== "active" && past(d.endedAt, 180 * DAY_MS)) delete r.devices[id];
  }
  for (const [id, i] of Object.entries(r.invites)) {
    if (!i) { delete r.invites[id]; continue; }
    if (i.state === "used" && !own(r.devices, i.used?.device)) delete r.invites[id];
    else if (i.state === "cancelled" && past(i.endedAt, 30 * DAY_MS)) delete r.invites[id];
    else if (i.state === "pending" && i.kind === "own_mac" && past(i.expiresAt, 30 * DAY_MS)) delete r.invites[id];
  }
}

/** At the 500-invite cap, the rows nothing live still needs go early, oldest first, rather
 * than locking every invite until pruning reaches them: cancelled invites, expired own-Mac
 * links, and used invites whose Mac has ended. Their late visitors get the nameless
 * "doesn't work" face (§2.3 already accepts that degradation for pruned rows). Pending
 * member invites and used invites behind an active Mac are never evicted. */
/** @param {any} r @param {number} now */
function evictInvites(r, now) {
  /** @type {[string, number][]} */
  const spare = [];
  for (const [id, i] of Object.entries(r.invites)) {
    if (!i) continue;
    const at = (/** @type {unknown} */ v) => (typeof v === "string" && Number.isFinite(Date.parse(v)) ? Date.parse(v) : 0);
    if (i.state === "cancelled") spare.push([id, at(i.endedAt)]);
    else if (i.state === "pending" && i.kind === "own_mac" && inviteState(i, now) === "expired") spare.push([id, at(i.expiresAt)]);
    else if (i.state === "used" && own(r.devices, i.used?.device)?.state !== "active") spare.push([id, at(i.used?.at)]);
  }
  spare.sort((a, b) => a[1] - b[1]);
  const room = Object.keys(r.invites).length - CAPS.invites + 1;
  for (const [id] of spare.slice(0, Math.max(0, room))) delete r.invites[id];
}

/** "Expired" is derived, never stored (§2.3). */
/** @param {any} invite @param {number} now */
const inviteState = (invite, now) => (invite.state === "pending" && now > Date.parse(invite.expiresAt) ? "expired" : invite.state);

/** @param {any} r @param {string} codeHash @returns {[string, any] | null} */
function inviteByHash(r, codeHash) {
  for (const [id, i] of Object.entries(r.invites)) if (i && i.codeHash === codeHash) return [id, i];
  return null;
}

/** @param {any} r */
const activeMembers = (r) => Object.values(r.members).filter((m) => m && m.role === "member" && m.state === "active").length;
/** @param {any} r */
const activeOwnerMacs = (r) => Object.values(r.devices).filter((d) => d && d.member === M1 && d.state === "active").length;
/** The token-holder flag (§3.2, H1): an active m1 Mac that proved a live token within 30 days. */
/** @param {any} d @param {number} now */
const tokenHolder = (d, now) => !!d && d.member === M1 && d.state === "active" && typeof d.proofAt === "string" && now - Date.parse(d.proofAt) <= TOKEN_FLAG_MS;

/** What a person typed (or a Mac's name): control characters stripped, NFC, trimmed;
 * null when it isn't text or comes out empty; a refusal when it's over `max` characters. */
/** @param {unknown} v @param {number} max */
function typed(v, max) {
  if (typeof v !== "string") return null;
  const s = v.replace(/\p{Cc}/gu, "").normalize("NFC").trim();
  if (!s) return null;
  if ([...s].length > max) throw bad();
  return s;
}
/** A Mac's own name: the same normalization, cut to length rather than refused. */
/** @param {unknown} v */
function macName(v) {
  if (typeof v !== "string") return null;
  const s = [...v.replace(/\p{Cc}/gu, "").normalize("NFC").trim()].slice(0, CAPS.name).join("").trim();
  return s || null;
}

/** @param {number} n */
function randomB64(n) {
  return toB64url(crypto.getRandomValues(new Uint8Array(n)));
}

// ───────── credentials (§3.1, §3.2) ─────────

/** The device key's two halves, the secret already hashed (only hashes are compared). */
/** @param {Request} request @returns {Promise<{ did: string, hash: string } | null>} */
async function bearer(request) {
  const m = (request.headers.get("authorization") ?? "").match(/^Bearer (d_[A-Za-z0-9_-]{16})\.([A-Za-z0-9_-]{16,128})$/);
  return m ? { did: m[1], hash: await sha256Hex(m[2]) } : null;
}

/**
 * The caller, checked against `roster` (§3.2): unknown → 401; a removed or left device or
 * member → 403 removed with why. The role is the roster's, never the request's.
 * @typedef {{ did: string, mid: string, device: any, member: any, ownerClass: boolean }} Caller
 * @param {any} roster @param {{ did: string, hash: string } | null} cred @returns {Caller}
 */
function caller(roster, cred) {
  const device = cred ? own(roster.devices, cred.did) : undefined;
  if (!cred || !device || device.verifier !== cred.hash) throw new Refusal(401, "unauthorized");
  const member = own(roster.members, device.member);
  if (!member) throw new Refusal(401, "unauthorized");
  if (device.state !== "active") throw new Refusal(403, "removed", { why: device.state === "left" ? "left" : "removed" });
  if (member.state !== "active") throw new Refusal(403, "removed", { why: member.state === "left" ? "left" : "removed" });
  return { did: cred.did, mid: device.member, device, member, ownerClass: member.role === "owner" };
}

/** The roster and the caller, from one read. */
/** @param {Request} request @param {Env} env */
async function authed(request, env) {
  const cred = await bearer(request);
  const read = await readRoster(env);
  return { read, cred, who: caller(read.roster, cred) };
}
/** @param {Caller} who */
function ownerClassOnly(who) {
  if (!who.ownerClass) throw new Refusal(403, "forbidden");
}

/** The mint parent (§3.4): the token's derived S3 pair, one secret binding. */
/** @param {Env} env @returns {{ account_id: string, bucket: string, key_id: string, secret: string } | null} */
function parentOf(env) {
  if (typeof env.R2_PARENT !== "string") return null;
  try {
    const p = JSON.parse(env.R2_PARENT);
    return ["account_id", "bucket", "key_id", "secret"].every((k) => typeof p?.[k] === "string" && p[k]) ? p : null;
  } catch {
    return null;
  }
}

/** The space's identity as a Mac stores it. */
/** @param {Env} env @param {URL} url */
function spaceOf(env, url) {
  const parent = parentOf(env);
  const parts = url.host.split(".");
  const workersDev = url.host.endsWith(".workers.dev") && parts.length === 4;
  return { host: url.host, account_id: parent?.account_id ?? null, bucket: parent?.bucket ?? null, label: parts[0], subdomain: workersDev ? parts[1] : null };
}

// ───────── the join machine (§4.4, §4.5) ─────────

/** @param {Env} env @param {any} b @param {number} now */
async function peek(env, b, now) {
  if (typeof b.code !== "string") throw bad();
  const { roster } = await readRoster(env);
  const hit = ID.test(b.code) ? inviteByHash(roster, await sha256Hex(b.code)) : null;
  if (!hit) throw new Refusal(404, "unknown");
  const [, invite] = hit;
  return answer({ state: inviteState(invite, now), kind: invite.kind, inviter: invite.inviter, expiresAt: invite.expiresAt, now: iso(now), iface: IFACE });
}

/** @param {Env} env @param {URL} url @param {any} b @param {number} now */
async function join(env, url, b, now) {
  const d = b.device;
  if (typeof b.code !== "string" || !d || typeof d !== "object" || typeof d.id !== "string" || !DEVICE_ID.test(d.id)
    || typeof d.verifier !== "string" || !HEX64.test(d.verifier)) throw bad();
  if (!ID.test(b.code)) throw new Refusal(404, "unknown");
  const name = macName(d.name);
  const p = b.previous;
  const previous = p && typeof p === "object" && typeof p.id === "string" && DEVICE_ID.test(p.id) && typeof p.secret === "string" && p.secret.length <= 128
    ? { id: p.id, hash: await sha256Hex(p.secret) } : null;
  const codeHash = await sha256Hex(b.code);
  const fresh = `m_${randomB64(12)}`;
  const at = iso(now);
  const s = spaceOf(env, url);
  /** @param {any} invite @param {string} member */
  const joined = (invite, member) => ({
    kind: invite.kind, role: member === M1 ? "owner" : "member", member, device: d.id, inviter: invite.inviter,
    space: { ...s, owner_name: invite.inviter, iface: IFACE },
  });
  const result = await mutate(env, now, (r) => {
    const hit = inviteByHash(r, codeHash);
    if (!hit) throw new Refusal(404, "unknown");
    const [iid, invite] = hit;
    const state = inviteState(invite, now);
    if (state === "cancelled") throw new Refusal(410, "cancelled");
    if (state === "expired") throw new Refusal(410, "expired");
    if (state === "used") {
      // A lost reply, replayed: the same Mac with the same verifier gets the same answer.
      if (invite.used?.device === d.id && invite.used?.verifier === d.verifier) return { result: joined(invite, invite.used.member) };
      throw new Refusal(409, "used");
    }
    if (state !== "pending") throw new Refusal(404, "unknown");
    // Re-invited after removal (H2): the same member comes back, under its old label.
    let revive = null;
    const old = previous ? own(r.devices, previous.id) : undefined;
    if (previous && old && old.verifier === previous.hash) {
      if (old.state === "active") throw new Refusal(409, "already-here");
      const m = own(r.members, old.member);
      if (old.member !== M1 && m && m.role === "member" && (m.state === "removed" || m.state === "left") && invite.kind === "member") revive = old.member;
    }
    if (own(r.devices, d.id)) throw new Refusal(409, "conflict");
    let member;
    if (invite.kind === "own_mac") {
      if (activeOwnerMacs(r) >= CAPS.ownerMacs) throw new Refusal(409, "full");
      member = M1;
    } else {
      if (activeMembers(r) >= CAPS.members) throw new Refusal(409, "full");
      if (revive) {
        const m = r.members[revive];
        // The row reads the re-join date (§4.5: "Mher — joined Oct 2"); the first one stays
        // in the history.
        Object.assign(m, { state: "active", endedAt: null, joinedAt: at, reinvites: [...(Array.isArray(m.reinvites) ? m.reinvites : []), { invite: iid, label: invite.label, at, joinedBefore: m.joinedAt ?? null }] });
        member = revive;
      } else {
        if (own(r.members, fresh)) throw new Refusal(503, "busy");
        r.members[fresh] = { role: "member", label: invite.label, inviter: invite.inviter, state: "active", joinedAt: at, endedAt: null, invite: iid, reinvites: [] };
        member = fresh;
      }
    }
    r.devices[d.id] = { member, name: invite.kind === "own_mac" ? name : null, verifier: d.verifier, state: "active", joinedAt: at, endedAt: null, via: iid, tokenKey: null, proofAt: null };
    invite.state = "used";
    invite.used = { at, member, device: d.id, verifier: d.verifier };
    return { write: true, result: joined(invite, member) };
  });
  return answer(result);
}

// ───────── enroll: a Mac holding the token (§3.2, §4.6) ─────────

/** @param {Request} request @param {Env} env @param {URL} url @param {number} now */
async function enroll(request, env, url, now) {
  const { value: b, bytes } = await rawBody(request);
  const proof = (request.headers.get("authorization") ?? "").match(/^ShroomParent (\d{1,12})\.([A-Za-z0-9_-]{16,128})$/);
  if (!proof) throw new Refusal(401, "unauthorized");
  const parent = parentOf(env);
  if (!parent) throw space("no-parent");
  if (Math.abs(now / 1000 - Number(proof[1])) > CLOCK_SKEW_SECONDS) throw new Refusal(401, "clock");
  // Domain-separated (no JWT signing input, which starts "eyJ", can be a proof), and bound
  // to the method, host, path, time and body.
  const msg = `shroom-parent-v1\n${request.method}\n${url.host}\n${url.pathname}\n${proof[1]}\n${await sha256Hex(bytes)}`;
  const key = await crypto.subtle.importKey("raw", new TextEncoder().encode(parent.secret), { name: "HMAC", hash: "SHA-256" }, false, ["verify"]);
  let ok = false;
  try { ok = await crypto.subtle.verify("HMAC", key, fromB64url(proof[2]), new TextEncoder().encode(msg)); } catch { ok = false; }
  if (!ok) throw new Refusal(401, "stale-parent");

  const d = b.device;
  if (!d || typeof d !== "object" || typeof d.id !== "string" || !DEVICE_ID.test(d.id) || typeof d.verifier !== "string" || !HEX64.test(d.verifier)) throw bad();
  const name = macName(d.name);
  // This Mac's stamp in this space (B144): sha-256 over its hardware, its account and the
  // space's host, made on the Mac — never the hardware's own id. Optional (older apps).
  if (d.machine !== undefined && d.machine !== null && (typeof d.machine !== "string" || !HEX64.test(d.machine))) throw bad();
  const machine = typeof d.machine === "string" ? d.machine : null;
  const ownerName = b.ownerName === undefined || b.ownerName === null ? null : typed(b.ownerName, CAPS.inviter);
  const at = iso(now);
  await mutate(env, now, (r) => {
    let write = false;
    const m1 = own(r.members, M1);
    if (!m1) {
      r.members[M1] = { role: "owner", name: ownerName, state: "active", joinedAt: at };
      write = true;
    } else if (ownerName !== null && m1.name !== ownerName) {
      m1.name = ownerName;
      write = true;
    }
    const dev = own(r.devices, d.id);
    const stamp = { state: "active", endedAt: null, tokenKey: parent.key_id, proofAt: at };
    // No m1 cap here: a token outranks it, as it outranks a Remove (U8) — a fresh token on
    // any Mac is always enough to recover (§3.7), even from a roster filled with Macs that
    // came in through a stolen own-Mac link.
    if (!dev) {
      r.devices[d.id] = { member: M1, name, verifier: d.verifier, joinedAt: at, via: "enroll", ...stamp, ...(machine ? { machine } : {}) };
      write = true;
      // The same Mac set up again under a new key — its folder went (Disconnect, an
      // uninstall, by hand) — leaves its old record behind: that one has left (B144). By
      // the stamp; a record from before stamps by its name, only a token Mac's that hasn't
      // proved the token for a day. A token Mac taken by mistake comes back at its next
      // enroll (below: a token outranks the state).
      for (const [id, x] of Object.entries(r.devices)) {
        if (id === d.id || !x || x.member !== M1 || x.state !== "active" || x.via !== "enroll") continue;
        const same = machine && x.machine ? x.machine === machine : !x.machine && name !== null && x.name === name && !(now - Date.parse(x.proofAt) <= RESTAMP_MS);
        if (same) Object.assign(x, { state: "left", endedAt: at });
      }
    } else {
      // Only ever under m1, and only for the Mac that holds this device's secret.
      if (dev.member !== M1 || dev.verifier !== d.verifier) throw new Refusal(409, "conflict");
      const stamped = machine && dev.machine !== machine ? { machine } : {};
      if (dev.state !== "active") {
        // A token proves more than a Remove… can take away (U8), and more than the cap.
        Object.assign(dev, stamp, { name }, stamped);
        write = true;
      } else if (dev.name !== name || typeof dev.proofAt !== "string" || now - Date.parse(dev.proofAt) > RESTAMP_MS || machine && dev.machine !== machine) {
        Object.assign(dev, stamp, { name }, stamped);
        write = true;
      }
    }
    return { write, result: null };
  });
  // The token Mac proved the parent: the space can take uploads again (§4.18).
  await setHealth(env, "ok", null, d.id, now);
  return answer({ member: M1, device: d.id, iface: IFACE, token: true });
}

// ───────── who am I (§4.7) ─────────

/** @param {Request} request @param {Env} env @param {URL} url @param {number} now */
async function me(request, env, url, now) {
  const { read, who } = await authed(request, env);
  const s = spaceOf(env, url);
  const m1 = own(read.roster.members, M1);
  const out = {
    you: { member: who.mid, device: who.did, role: who.ownerClass ? "owner" : "member", name: typeof who.device.name === "string" ? who.device.name : null },
    space: { host: s.host, account_id: s.account_id, bucket: s.bucket, owner_name: who.ownerClass ? (m1?.name ?? null) : (who.member.inviter ?? null), iface: IFACE },
  };
  if (!who.ownerClass) return answer(out);
  return answer({ ...out, owner: { mark: await markOn(env), tokenHolders: Object.values(read.roster.devices).filter((d) => tokenHolder(d, now)).length, token: tokenHolder(who.device, now) } });
}

// ───────── recording credentials (§3.4, §4.8) ─────────

/** @param {Request} request @param {Env} env @param {any} b @param {number} now */
async function mint(request, env, b, now) {
  const { who } = await authed(request, env);
  // Checked before anything reaches a key or a claim (H6).
  const rec = b.rec;
  const purpose = b.purpose;
  if (typeof rec !== "string" || !ID.test(rec) || (purpose !== "upload" && purpose !== "manage")) throw bad();
  const [ledger, meta] = await Promise.all([ledgerGet(env, rec), metaFact(env, rec)]);
  let owner = ledger ? ledger.member : meta.exists ? M1 : null;
  if (purpose === "manage") {
    if (owner !== null && owner !== who.mid && !who.ownerClass) throw new Refusal(403, "not-yours");
    if (!meta.exists || owner === null) throw new Refusal(404, "absent");
    // A tombstone is final for its maker: a member's manage credential would let them write
    // the meta back to live. Owner-class may still finish a delete; the token Mac's sweep
    // reaps any bytes a member's interrupted delete left behind.
    if (meta.deleted && !who.ownerClass) throw new Refusal(410, "deleted");
  } else {
    if (owner !== null && owner !== who.mid) throw new Refusal(403, "not-yours");
    if (meta.deleted) throw new Refusal(410, "deleted");
    if (owner === null) {
      // A first mint: the budget, then the ledger — before the parent is read, so even a
      // blocked first mint records whose video it is (§4.18).
      await spendBudget(env, who.did, now);
      const entry = await ledgerCreate(env, rec, { v: 1, member: who.mid, device: who.did, at: iso(now) });
      if (entry.member !== who.mid) throw new Refusal(403, "not-yours");
      owner = who.mid;
    }
  }
  const parent = parentOf(env);
  if (!parent) {
    try { await setHealth(env, "blocked", "no-parent", who.did, now); } catch { /* advisory */ }
    throw space("no-parent");
  }
  const iat = Math.floor(now / 1000);
  const ttl = purpose === "upload" ? TTL.upload : TTL.manage;
  const jwt = await memberJwt(parent, rec, iat, ttl);
  return answer({
    accessKeyId: parent.key_id, secretAccessKey: await sha256Hex(jwt), sessionToken: btoa(`jwt/${jwt}`),
    expiresAt: iso((iat + ttl) * 1000), bucket: parent.bucket, endpoint: `https://${parent.account_id}.r2.cloudflarestorage.com`, owner,
  });
}

/** The JWT exactly as `shroom-cloud::mint::jwt` signs it (B32): the claims in serde_json's
 * alphabetical order, nested objects included — a fixed literal, so the two signers agree
 * byte for byte (the golden vector, C1). Exactly two paths, both the request's own id. */
/** @param {{ account_id: string, bucket: string, key_id: string, secret: string }} p @param {string} rec @param {number} iat @param {number} ttl */
export async function memberJwt(p, rec, iat, ttl) {
  const s = JSON.stringify;
  const header = `{"alg":"HS256","typ":"JWT"}`;
  const claims = `{"aud":${s(`${p.account_id}.r2.cloudflarestorage.com`)},"bucket":${s(p.bucket)},"exp":${iat + ttl},"iat":${iat},"iss":${s(p.key_id)},`
    + `"paths":{"objectPaths":[${s(`meta/${rec}.json`)}],"prefixPaths":[${s(`rec/${rec}/`)}]},"scope":"object-read-write","sub":${s(p.account_id)}}`;
  const input = `${toB64url(new TextEncoder().encode(header))}.${toB64url(new TextEncoder().encode(claims))}`;
  return `${input}.${toB64url(await hmac(new TextEncoder().encode(p.secret), input))}`;
}

/** Whether a record exists and is a tombstone (an unreadable one exists, alive). */
/** @param {Env} env @param {string} id */
async function metaFact(env, id) {
  const [object, gone] = await Promise.all([env.BUCKET.get(`meta/${id}.json`), env.BUCKET.head(`${GONE}${id}`)]);
  if (gone) {
    try { await object?.body?.cancel(); } catch { /* nothing to release */ }
    return { exists: true, deleted: true, sealed: true };
  }
  if (!object) return { exists: false, deleted: false };
  try { return { exists: true, deleted: JSON.parse(await object.text())?.state === "deleted" }; } catch { return { exists: true, deleted: false }; }
}

/** The ownership ledger (§2.4): `{v, member, device, at}`, the member also in custom
 * metadata so a listing maps ids to members without a read each. Immutable. */
/** @param {Env} env @param {string} id @returns {Promise<{ member: string } | null>} */
async function ledgerGet(env, id) {
  if (!ID.test(id)) return null;
  const object = await env.BUCKET.get(`${LEDGER}${id}`);
  if (!object) return null;
  try {
    const v = JSON.parse(await object.text());
    if (typeof v?.member === "string") return v;
  } catch { /* fall back to the metadata */ }
  const m = object.customMetadata?.m;
  return typeof m === "string" ? { member: m } : null;
}

/** Create-only: on a conflict, whoever got there first — the caller's own entry included. */
/** @param {Env} env @param {string} id @param {{ v: number, member: string, device: string, at: string }} entry */
async function ledgerCreate(env, id, entry) {
  for (let attempt = 0; attempt < 3; attempt++) {
    if (await casPut(env, `${LEDGER}${id}`, JSON.stringify(entry), { etagDoesNotMatch: "*" }, { m: entry.member })) return entry;
    const existing = await ledgerGet(env, id);
    if (existing) return existing;
  }
  throw new Refusal(503, "busy");
}

/** The upload budget (H4): one small counter per device, outside the roster, by CAS. */
/** @param {Env} env @param {string} did @param {number} now */
async function spendBudget(env, did, now, field = "n", budget = UPLOAD_BUDGET) {
  const key = `${RATE}${did}`;
  const window = iso(Math.floor(now / HOUR_MS) * HOUR_MS);
  for (let attempt = 0; attempt < CAS_TRIES; attempt++) {
    const object = await env.BUCKET.get(key);
    /** @type {any} */
    let cur = null;
    if (object) { try { cur = JSON.parse(await object.text()); } catch { cur = null; } }
    // One object per device and window: uploads count in `n`, page keys in `k` (B125).
    const same = cur && cur.window === window;
    const count = (/** @type {string} */ f) => (same && Number.isInteger(cur[f]) ? cur[f] : 0);
    const n = count(field);
    if (n >= budget) throw new Refusal(429, "too-many", { retryAfter: Math.max(1, Math.ceil((Date.parse(window) + HOUR_MS - now) / 1000)) });
    /** @type {Record<string, any>} */
    const next = { window, n: count("n") };
    if (field === "k" || count("k") > 0) next.k = count("k");
    next[field] = n + 1;
    if (await casPut(env, key, JSON.stringify(next), object ? { etagMatches: object.etag } : { etagDoesNotMatch: "*" })) return;
  }
  throw new Refusal(503, "busy");
}

// ───────── the one-time page key (§3.3, §4.9) ─────────

/** @param {Request} request @param {Env} env @param {any} b @param {number} now */
async function key(request, env, b, now) {
  const { who } = await authed(request, env);
  if (typeof b.kHash !== "string" || !HEX64.test(b.kHash)) throw bad();
  const name = `${KEYS}${b.kHash}`;
  await spendBudget(env, who.did, now, "k", KEY_BUDGET);
  const grant = { member: who.mid, device: who.did, exp: iso(now + KEY_SECONDS * 1000) };
  if (await casPut(env, name, JSON.stringify(grant), { etagDoesNotMatch: "*" })) return answer({ exp: grant.exp });
  const object = await env.BUCKET.get(name);
  /** @type {any} */
  let existing = null;
  if (object) { try { existing = JSON.parse(await object.text()); } catch { existing = null; } }
  // A retry after a lost reply finds its own grant; any other grant is someone else's.
  if (existing && existing.member === who.mid && existing.device === who.did && !existing.spent) return answer({ exp: existing.exp });
  throw new Refusal(409, "conflict");
}

// ───────── the Library for Macs without the token (§4.10) ─────────

/** One page of a listing that never trusts the count (P3): a page that comes back empty
 * but truncated is followed by its cursor. */
/** @param {Env} env @param {string} prefix @param {string | undefined} startAfter @param {boolean} withMeta @returns {Promise<R2Objects>} */
async function listPage(env, prefix, startAfter, withMeta) {
  /** @type {R2ListOptions} */
  let options = { prefix, limit: 1000 };
  if (startAfter !== undefined) options.startAfter = startAfter;
  if (withMeta) options.include = ["customMetadata"];
  for (let i = 0; i < 5; i++) {
    const page = await env.BUCKET.list(options);
    if (page.objects.length || !page.truncated || !page.cursor) return page;
    options = { prefix, limit: 1000, cursor: page.cursor, ...(withMeta ? { include: ["customMetadata"] } : {}) };
  }
  // Still truncated after the follow budget: never make up an end of the list (a client
  // would read every cached id past `after` as gone). A silent backoff instead.
  throw new Refusal(503, "busy");
}
/** @param {string} key */
const ledgerIdOf = (key) => { const id = key.slice(LEDGER.length); return key.startsWith(LEDGER) && ID.test(id) ? id : null; };
/** @param {string} key */
const metaIdOf = (key) => key.match(/^meta\/([A-Za-z0-9_-]{16})\.json$/)?.[1] ?? null;
/** @param {R2Objects} page @param {(key: string) => string | null} idOf */
function lastId(page, idOf) {
  for (let i = page.objects.length - 1; i >= 0; i--) { const id = idOf(page.objects[i].key); if (id) return id; }
  return null;
}

/** The id → member map for a page's ledger entries. An entry missing its metadata (an
 * S3 write that left out `x-amz-meta-m`) costs a GET: at most LIBRARY_BODIES of them, in
 * parallel; past that, `cut` is the last id resolved and the page ends there. */
/** @param {Env} env @param {R2Objects} page @param {(id: string) => boolean} inPage */
async function ledgerOwners(env, page, inPage) {
  /** @type {Map<string, string>} */
  const owners = new Map();
  /** @type {string[]} */
  const missing = [];
  for (const o of page.objects) {
    const id = ledgerIdOf(o.key);
    if (!id || !inPage(id)) continue;
    const m = o.customMetadata?.m;
    if (typeof m === "string") owners.set(id, m);
    else missing.push(id);
  }
  /** @type {string | null} */
  let cut = null;
  if (missing.length > LIBRARY_BODIES) {
    cut = missing[LIBRARY_BODIES - 1];
    missing.length = LIBRARY_BODIES;
  }
  const read = await Promise.all(missing.map((id) => ledgerGet(env, id)));
  missing.forEach((id, i) => { const m = read[i]?.member; if (typeof m === "string") owners.set(id, m); });
  if (cut !== null) for (const id of [...owners.keys()]) if (id > cut) owners.delete(id);
  return { owners, cut };
}

/** @param {Request} request @param {Env} env @param {any} b */
async function library(request, env, b) {
  const { who } = await authed(request, env);
  const scope = b.scope;
  if (scope !== "mine" && scope !== "everyone" && scope !== "owners") throw bad();
  if (!who.ownerClass && scope !== "mine") throw new Refusal(403, "forbidden");
  const idOrNull = (/** @type {unknown} */ v) => { if (v === null || v === undefined) return null; if (typeof v === "string" && ID.test(v)) return v; throw bad(); };
  const after = idOrNull(b.after);
  const knownThrough = idOrNull(b.knownThrough);
  /** @type {Record<string, string>} */
  const known = {};
  if (b.known && typeof b.known === "object") {
    for (const [id, etag] of Object.entries(b.known)) if (ID.test(id) && typeof etag === "string") known[id] = etag;
  }

  if (scope === "owners") {
    // The ledger walk alone: the id → member map, a page at a time, uncapped (H3).
    const page = await listPage(env, LEDGER, after ? `${LEDGER}${after}` : undefined, true);
    const last = page.truncated ? lastId(page, ledgerIdOf) : null;
    const resolved = await ledgerOwners(env, page, (id) => (!after || id > after) && (!last || id <= last));
    return answer({ owners: Object.fromEntries(resolved.owners), next: resolved.cut ?? last });
  }

  // The ledger and the records sort by the same id: walk them in lockstep (D).
  const [ledgers, metas] = await Promise.all([
    listPage(env, LEDGER, after ? `${LEDGER}${after}` : undefined, true),
    listPage(env, "meta/", after ? `meta/${after}.json` : undefined, false),
  ]);
  /** @type {string | null} */
  let end = null;
  const bound = (/** @type {string | null} */ id) => { if (id !== null && (end === null || id < end)) end = id; };
  if (ledgers.truncated) bound(lastId(ledgers, ledgerIdOf));
  if (metas.truncated) bound(lastId(metas, metaIdOf));
  bound(knownThrough); // never judge past the slice the client sent
  const inPage = (/** @type {string} */ id) => (!after || id > after) && (end === null || id <= end);
  const { owners: ledgerOwner, cut: ledgerCut } = await ledgerOwners(env, ledgers, inPage);
  bound(ledgerCut);
  /** @type {{ id: string, etag: string, owner: string, meta?: unknown }[]} */
  let items = [];
  for (const o of metas.objects) {
    const id = metaIdOf(o.key);
    if (!id || !inPage(id)) continue;
    const owner = ledgerOwner.get(id) ?? M1; // a record without a ledger entry is member #1's
    if (scope === "mine" && owner !== who.mid) continue;
    items.push({ id, etag: o.etag, owner });
  }
  // Bodies only for what changed, at most 40 a call: past the 40th, the page ends there.
  const changed = items.filter((i) => known[i.id] !== i.etag);
  if (changed.length > LIBRARY_BODIES) {
    const cut = changed[LIBRARY_BODIES - 1].id;
    end = cut;
    items = items.filter((i) => i.id <= cut);
  }
  await Promise.all(items.filter((i) => known[i.id] !== i.etag).map(async (i) => {
    const [object, gone] = await Promise.all([env.BUCKET.get(`meta/${i.id}.json`), env.BUCKET.head(`${GONE}${i.id}`)]);
    /** @type {any} */
    let meta = null;
    if (object) { try { meta = JSON.parse(await object.text()); } catch { meta = null; } }
    // A deletion record outranks whatever the meta says now (B125): a sealed id lists as
    // the tombstone the Mac writes, whatever a stale credential put there since.
    if (gone) meta = { schema: 1, id: i.id, state: "deleted", createdAt: typeof meta?.createdAt === "string" ? meta.createdAt : null };
    i.meta = meta;
  }));
  return answer({ items, next: end });
}

// ───────── Members (§4.11–§4.15) ─────────

/** @param {any} invite @param {string} id @param {number} now */
const inviteView = (invite, id, now) => ({ id, kind: invite.kind, label: invite.label ?? null, createdAt: invite.createdAt, expiresAt: invite.expiresAt, expired: now > Date.parse(invite.expiresAt) });

/** @param {Request} request @param {Env} env @param {number} now */
async function members(request, env, now) {
  const { read, who } = await authed(request, env);
  ownerClassOnly(who);
  const r = read.roster;
  const entries = (/** @type {any} */ t) => /** @type {[string, any][]} */ (Object.entries(t).filter(([, v]) => v && typeof v === "object"));
  /** @type {Record<string, string>} */
  const labels = {};
  for (const [id, m] of entries(r.members)) if (m.role === "member") labels[id] = m.label;
  return answer({
    now: iso(now),
    owner: {
      id: M1, name: own(r.members, M1)?.name ?? null,
      devices: entries(r.devices).filter(([, d]) => d.member === M1 && d.state === "active")
        .map(([id, d]) => ({ id, name: d.name ?? null, joinedAt: d.joinedAt, token: tokenHolder(d, now), self: id === who.did })),
    },
    members: entries(r.members).filter(([, m]) => m.role === "member" && m.state === "active").map(([id, m]) => ({ id, label: m.label, joinedAt: m.joinedAt })),
    invites: entries(r.invites).filter(([, i]) => i.state === "pending").map(([id, i]) => inviteView(i, id, now)),
    labels,
  });
}

/** Invite, and [ New link ] (§4.12). The server sets the clock; the app builds the link. */
/** @param {Request} request @param {Env} env @param {any} b @param {number} now */
async function invites(request, env, b, now) {
  const { read, cred, who } = await authed(request, env);
  ownerClassOnly(who);
  if (typeof b.id !== "string" || !INVITE_ID.test(b.id) || typeof b.codeHash !== "string" || !HEX64.test(b.codeHash)) throw bad();
  const at = iso(now);
  if (b.renew === true) {
    const out = await mutate(env, now, (r) => {
      ownerClassOnly(caller(r, cred));
      const invite = own(r.invites, b.id);
      if (!invite) throw new Refusal(404, "unknown");
      if (invite.state === "used") throw new Refusal(409, "used");
      if (invite.state === "cancelled") throw new Refusal(410, "cancelled");
      if (invite.codeHash === b.codeHash) return { result: inviteView(invite, b.id, now) };
      if (inviteByHash(r, b.codeHash)) throw new Refusal(409, "conflict");
      const live = Object.values(r.invites).filter((i) => i && i.state === "pending" && now <= Date.parse(i.expiresAt)).length;
      if (inviteState(invite, now) === "expired" && live >= CAPS.pending) throw new Refusal(409, "full");
      Object.assign(invite, { codeHash: b.codeHash, createdAt: at, expiresAt: iso(now + (invite.kind === "own_mac" ? INVITE_SECONDS.own_mac : INVITE_SECONDS.member) * 1000) });
      return { write: true, result: inviteView(invite, b.id, now) };
    }, read);
    return answer({ invite: out });
  }
  const kind = b.kind;
  if (kind !== "member" && kind !== "own_mac") throw bad();
  const label = kind === "member" ? typed(b.label, CAPS.label) : null;
  const inviter = typed(b.inviter, CAPS.inviter);
  if ((kind === "member" && label === null) || inviter === null) throw bad();
  const out = await mutate(env, now, (r) => {
    ownerClassOnly(caller(r, cred));
    const existing = own(r.invites, b.id);
    if (existing) {
      // Idempotent on (id, codeHash): a retried create gets the row it made.
      if (existing.codeHash === b.codeHash) return { result: { status: 200, invite: inviteView(existing, b.id, now) } };
      throw new Refusal(409, "conflict");
    }
    if (inviteByHash(r, b.codeHash)) throw new Refusal(409, "conflict");
    const pending = Object.values(r.invites).filter((i) => i && i.state === "pending" && now <= Date.parse(i.expiresAt)).length;
    if (pending >= CAPS.pending) throw new Refusal(409, "full");
    if (Object.keys(r.invites).length >= CAPS.invites) evictInvites(r, now);
    if (Object.keys(r.invites).length >= CAPS.invites) throw new Refusal(409, "full");
    const invite = { codeHash: b.codeHash, kind, label, inviter, by: who.did, createdAt: at, expiresAt: iso(now + (kind === "own_mac" ? INVITE_SECONDS.own_mac : INVITE_SECONDS.member) * 1000), state: "pending", used: null, endedAt: null };
    r.invites[b.id] = invite;
    return { write: true, result: { status: 201, invite: inviteView(invite, b.id, now) } };
  }, read);
  return answer({ invite: out.invite }, out.status);
}

/** Cancel (§4.13): the link stops working right away; the record stays for the page. */
/** @param {Request} request @param {Env} env @param {string} iid @param {number} now */
async function cancelInvite(request, env, iid, now) {
  const { read, cred, who } = await authed(request, env);
  ownerClassOnly(who);
  if (!INVITE_ID.test(iid)) throw new Refusal(404, "unknown");
  await mutate(env, now, (r) => {
    ownerClassOnly(caller(r, cred));
    const invite = own(r.invites, iid);
    if (!invite) throw new Refusal(404, "unknown");
    if (invite.state === "used") throw new Refusal(409, "used", { member: invite.used?.member ?? null });
    if (invite.state === "cancelled") return { result: null };
    Object.assign(invite, { state: "cancelled", endedAt: iso(now) });
    return { write: true, result: null };
  }, read);
  return noContent();
}

/** Remove a member (§4.14): every Mac of theirs with them; the label stays forever. */
/** @param {Request} request @param {Env} env @param {string} mid @param {number} now */
async function removeMember(request, env, mid, now) {
  const { read, cred, who } = await authed(request, env);
  ownerClassOnly(who);
  if (mid === M1) throw new Refusal(409, "owner");
  if (!MEMBER_ID.test(mid)) throw new Refusal(404, "unknown");
  await mutate(env, now, (r) => {
    ownerClassOnly(caller(r, cred));
    const m = own(r.members, mid);
    if (!m || m.role !== "member") throw new Refusal(404, "unknown");
    if (m.state !== "active") return { result: null };
    const at = iso(now);
    Object.assign(m, { state: "removed", endedAt: at });
    for (const d of Object.values(r.devices)) if (d && d.member === mid && d.state === "active") Object.assign(d, { state: "removed", endedAt: at });
    return { write: true, result: null };
  }, read);
  return noContent();
}

/** Remove another owner Mac, or leave (`self`) — §4.15, §16 #57. */
/** @param {Request} request @param {Env} env @param {string} target @param {number} now */
async function removeDevice(request, env, target, now) {
  const at = iso(now);
  if (target === "self") {
    // Idempotent (§4.15, §2.1): the device key alone authenticates, so a retried Leave
    // after a lost reply — the device already left or removed — is the same 204.
    const cred = await bearer(request);
    const read = await readRoster(env);
    await mutate(env, now, (r) => {
      const device = cred ? own(r.devices, cred.did) : undefined;
      if (!cred || !device || device.verifier !== cred.hash || !own(r.members, device.member)) throw new Refusal(401, "unauthorized");
      if (device.state !== "active") return { result: null };
      const me = { mid: device.member, device, member: r.members[device.member] };
      Object.assign(me.device, { state: "left", endedAt: at });
      // A member's last Mac takes the member with it; the label stays. m1 never changes.
      const others = Object.values(r.devices).some((d) => d && d.member === me.mid && d.state === "active");
      if (me.mid !== M1 && me.member.role === "member" && !others) Object.assign(me.member, { state: "left", endedAt: at });
      return { write: true, result: null };
    }, read);
    return noContent();
  }
  const { read, cred, who } = await authed(request, env);
  ownerClassOnly(who);
  if (target === who.did) throw new Refusal(409, "self");
  if (!DEVICE_ID.test(target)) throw new Refusal(404, "unknown");
  await mutate(env, now, (r) => {
    const me = caller(r, cred);
    ownerClassOnly(me);
    const d = own(r.devices, target);
    if (!d) throw new Refusal(404, "unknown");
    if (d.member !== M1) throw new Refusal(409, "not-owner-mac");
    if (d.state !== "active") return { result: null };
    if (tokenHolder(d, now)) throw new Refusal(409, "token-holder");
    Object.assign(d, { state: "removed", endedAt: at });
    return { write: true, result: null };
  }, read);
  return noContent();
}

/** The mark switch for owner-class Macs without the token (§4.16). */
/** @param {Request} request @param {Env} env @param {any} b @param {number} now */
async function mark(request, env, b, now) {
  const { who } = await authed(request, env);
  ownerClassOnly(who);
  if (typeof b.on !== "boolean") throw bad();
  await writeMark(env, b.on, now);
  return answer({ on: b.on });
}

// ───────── upload health — "Waiting for your space" (§4.18) ─────────

/** @param {Env} env @returns {Promise<any>} */
async function readHealth(env) {
  const object = await env.BUCKET.get(HEALTH);
  if (!object) return null;
  try { return JSON.parse(await object.text()); } catch { return null; }
}

/** Writes only on a change: a stream of identical reports costs reads, never writes. An
 * unconditional put — the record is advisory, and the last true report wins. */
/** @param {Env} env @param {"ok" | "blocked"} uploads @param {string | null} why @param {string | null} by @param {number} now */
async function setHealth(env, uploads, why, by, now) {
  const current = await readHealth(env);
  if ((current?.uploads === "blocked" ? "blocked" : "ok") === uploads) return;
  await env.BUCKET.put(HEALTH, JSON.stringify({ v: 1, uploads, since: iso(now), why, by }), { httpMetadata: JSON_TYPE });
}

/** @param {Request} request @param {Env} env @param {any} b @param {number} now */
async function health(request, env, b, now) {
  const { who } = await authed(request, env);
  if (typeof b.rec !== "string" || !ID.test(b.rec) || (b.uploads !== "refused" && b.uploads !== "ok")) throw bad();
  // Only the recording's own maker (or the owner) can say its uploads are refused: any
  // member flipping the space's waiting line for everyone else's pages was the gap (B125).
  if (b.uploads === "refused" && !who.ownerClass && (await ledgerGet(env, b.rec))?.member !== who.mid) throw new Refusal(403, "not-yours");
  if (b.uploads === "refused") await setHealth(env, "blocked", "refused", who.did, now);
  else await setHealth(env, "ok", null, who.did, now);
  return noContent();
}

/** `POST gone` (B125): after a delete, the Mac that made it seals it — the Worker records
 * the deletion where no recording credential reaches. Only once the record reads as a
 * tombstone (or is gone), only by its maker or the owner, and only on an id someone
 * recorded: sealing one nobody has would refuse its maker's first mint. Idempotent. */
/** @param {Request} request @param {Env} env @param {any} b @param {number} now */
async function gone(request, env, b, now) {
  const { who } = await authed(request, env);
  if (typeof b.rec !== "string" || !ID.test(b.rec)) throw bad();
  const [ledger, meta] = await Promise.all([ledgerGet(env, b.rec), metaFact(env, b.rec)]);
  if (meta.sealed) return noContent();
  const owner = ledger ? ledger.member : meta.exists ? M1 : null;
  if (owner === null) throw new Refusal(404, "absent");
  if (owner !== who.mid && !who.ownerClass) throw new Refusal(403, "not-yours");
  if (meta.exists && !meta.deleted) throw new Refusal(409, "not-deleted");
  await env.BUCKET.put(`${GONE}${b.rec}`, JSON.stringify({ v: 1, by: who.did, at: iso(now) }), { httpMetadata: JSON_TYPE });
  return noContent();
}

// ───────── the join page (§4.3, r6: join-landing and its faces) ─────────

/** Every face is 200 text/html (chat unfurlers always see a hero), noindex, no-store, no
 * referrer (the code is in the path) and unframeable. Opening never consumes; at most
 * one roster read. Whatever a person typed renders escaped, quotes included (H10). */
/** @param {Request} request @param {Env} env @param {URL} url @param {string} code @param {number} now */
async function joinPage(request, env, url, code, now) {
  const ua = request.headers.get("user-agent") ?? "";
  /** @type {{ title: string, blocks: string, script?: string }} */
  let face;
  if (!ID.test(code)) {
    face = lineFace(COPY.inviteUnknown);
  } else {
    /** @type {any} */
    let roster = null;
    try { roster = (await readRoster(env)).roster; } catch { roster = null; }
    const hit = roster ? inviteByHash(roster, await sha256Hex(code)) : null;
    if (!roster) face = lineFace(COPY.inviteTransient); // never "ask for a fresh one" (must-fix 4)
    else if (!hit) face = lineFace(COPY.inviteUnknown);
    else {
      const invite = hit[1];
      const inviter = typeof invite.inviter === "string" ? invite.inviter : "";
      const ownMac = invite.kind === "own_mac";
      const state = inviteState(invite, now);
      /** @type {{ member: string, device: string, ownerClass: boolean } | null} */
      let viewer = null;
      try { viewer = checkCookie(await cookieClaims(request, env), roster); } catch { viewer = null; }
      if (state === "pending") face = landingFace(invite, url.host, code, viewer?.ownerClass === true, /iPhone|iPad|iPod|Android|Windows|Linux|CrOS/.test(ua));
      else if (state === "used") {
        // "You're in" only to the one it let in. Every owner-class cookie is m1, so an
        // own-Mac link asks for the Mac it made, not the member; anyone else reads the
        // own-Mac words — the owner made the link, so "ask Bagrat" would be Bagrat's name
        // on Bagrat's screen.
        const them = ownMac ? viewer?.device === invite.used?.device : viewer?.member === invite.used?.member;
        face = lineFace(viewer && them ? COPY.youreIn : ownMac ? COPY.ownUsed : inviteUsed(inviter));
      }
      else face = lineFace(ownMac ? COPY.ownExpired : inviteExpired(inviter)); // expired or cancelled
    }
  }
  const res = html(joinShell(face));
  return request.method === "HEAD" ? new Response(null, { status: res.status, headers: res.headers }) : res;
}

/** A face that is its line alone (expired, used, unknown, transient): the line is the hero. */
/** @param {string} line */
function lineFace(line) {
  return { title: line, blocks: `<h1 class="alone">${escapeHtml(line)}</h1>` };
}

/** join-landing, its own-Mac variant, the owner view and the -elsewhere face. */
/** @param {any} invite @param {string} host @param {string} code @param {boolean} ownerView @param {boolean} elsewhere */
function landingFace(invite, host, code, ownerView, elsewhere) {
  const ownMac = invite.kind === "own_mac";
  const inviter = typeof invite.inviter === "string" ? invite.inviter : "";
  const hero = ownMac ? COPY.ownHero : joinHero(inviter);
  const link = `${host}/join/${code}`;
  // The label is admin-private: only an owner-class cookie, checked live, sees it.
  const label = typeof invite.label === "string" && invite.label ? invite.label : null;
  const note = ownerView ? `<p class="note">${escapeHtml(ownMac || !label ? COPY.ownOwnerNote : ownerNote(label))}</p>` : "";
  const away = `<div id="get"><section class="card"><p class="line">${escapeHtml(COPY.elsewhere)}</p><button class="btn primary wide" type="button" data-copy="link">${escapeHtml(COPY.copyLink)}</button></section></div>`;
  // "Read it first" opens the script the command runs, as the landing's does.
  const [readFirst, readRest] = COPY.readFirst.split(/(?= — )/);
  const get = `<div id="get"><p class="line">${escapeHtml(ownMac ? ownSame(host) : joinIntro(inviter))}</p>
<section class="card"><h2>${escapeHtml(COPY.getShroom)}</h2><p class="line">${escapeHtml(COPY.pasteThis)}</p>
<div class="cmd"><code id="cmd">${escapeHtml(installLine(link))}</code><button class="btn primary" type="button" data-copy="cmd">${escapeHtml(COPY.copy)}</button></div>
<div class="fine"><p><a href="https://shroom.am/install">${escapeHtml(readFirst)}</a>${escapeHtml(readRest)}</p><p>${escapeHtml(COPY.macos)}</p></div>
<div class="brew"><p class="line">${escapeHtml(COPY.useHomebrew)}</p><div class="cmd"><code id="brew">${escapeHtml(brewLine(link))}</code><button class="btn" type="button" data-copy="brew">${escapeHtml(COPY.copy)}</button></div></div></section>
<section class="have"><div class="ask"><p>${escapeHtml(COPY.alreadyHave)}</p><a class="btn" href="${escapeHtml(`shroom://join/${host}/${code}`)}">${escapeHtml(COPY.openShroom)}</a></div>
<p class="small">${escapeHtml(COPY.openFallback)}</p></section></div>`;
  const cfg = JSON.stringify({ flip: !elsewhere, away, copied: COPY.copied });
  // iPadOS asks for the desktop site with a Macintosh user agent; touch gives it away.
  const script = `<script>${COPY_JS}
(function(){var c=${jsonForScript(cfg)};
if(c.flip&&navigator.maxTouchPoints>1){var g=document.getElementById("get");if(g)g.outerHTML=c.away}
document.addEventListener("click",function(e){var b=e.target&&e.target.closest?e.target.closest("button[data-copy]"):null;if(!b)return;
var k=b.getAttribute("data-copy"),cmd=k==="link"?null:document.getElementById(k),t=cmd?cmd.textContent:location.origin+location.pathname,o=b.textContent;
shroomCopy(t,cmd).then(function(ok){if(!ok)return;b.textContent=c.copied;setTimeout(function(){b.textContent=o},1500)})})})();</script>`;
  return {
    title: hero,
    blocks: `${note}<h1>${escapeHtml(hero)}</h1>${elsewhere ? away : get}<p class="small">${escapeHtml(ownMac ? COPY.ownHour : COPY.onePerson)}</p>`,
    script,
  };
}

/** The join page: shroom's name at the top, linking home, and the mark bottom-right, on
 * every face whatever the space's switch says (U6) — it isn't a video page. */
/** @param {{ title: string, blocks: string, script?: string }} face */
function joinShell(face) {
  return `<!doctype html><html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1">
<meta name="robots" content="noindex"><title>${escapeHtml(face.title)}</title>${TAB_ICON_LINKS}<style>${TOKENS}${BASE}${JOIN_STYLE}</style></head>
<body><header class="top"><a class="brand" href="https://shroom.am">${LOGO}<span>shroom</span></a></header><main class="join">${face.blocks}</main>
<div class="end"><a class="made" href="https://shroom.am" title="${COPY.mark}">${LOGO}<span>${escapeHtml(COPY.mark)}</span></a></div>${face.script ?? ""}</body></html>`;
}

const JOIN_STYLE = `body{padding:0 16px 24px;font-size:15px;line-height:1.55}
.top,.join,.end{width:100%;max-width:640px}.top{min-height:64px;display:flex;align-items:center}
.brand{display:flex;align-items:center;gap:9px;min-height:44px;color:var(--text);font-size:18px;font-weight:600;text-decoration:none}.brand:hover{color:var(--text)}.brand svg{display:block;width:24px;height:26px}
.join{flex:1 0 auto;padding-top:40px;display:flex;flex-direction:column;gap:28px}.join p{margin:0}
.join h1{margin:0;font-size:1.75rem;font-weight:600;line-height:1.25;overflow-wrap:anywhere}.join h1.alone{font-size:1.25rem;font-weight:500;line-height:1.45;max-width:460px}
.join:has(>h1.alone){padding-top:0;min-height:320px;align-items:center;justify-content:center;text-align:center}
.line{color:var(--muted)}.small{color:var(--quiet);font-size:13px}.note{display:flex;gap:8px;align-items:flex-start;margin-bottom:-12px;padding:10px 12px;border-radius:9px;background:color-mix(in srgb,var(--accent) 16%,var(--bg));color:var(--muted);font-size:14px}.note::before{content:"";flex:none;width:8px;height:8px;margin-top:6px;border-radius:4px;background:var(--accent)}
#get{display:flex;flex-direction:column;gap:28px}#get>.line:first-child{margin-top:-16px}
.card{background:var(--card);border:1px solid var(--card-edge);border-radius:12px;padding:20px;display:flex;flex-direction:column;gap:14px}
.card h2{margin:0;font-size:1.0625rem;font-weight:600}
.cmd{display:flex;flex-wrap:wrap;align-items:stretch;gap:8px}
.cmd code{flex:1 1 280px;min-width:0;padding:12px;border-radius:8px;background:var(--well);color:var(--text);font:13px/1.5 ui-monospace,SFMono-Regular,Menlo,monospace;overflow-wrap:anywhere}
.fine{display:flex;flex-direction:column;gap:2px;color:var(--quiet);font-size:13px}
.brew{border-top:1px solid var(--hair);padding-top:14px;display:flex;flex-direction:column;gap:8px}
.have{border-top:1px solid var(--hair);padding-top:24px;display:flex;flex-direction:column;gap:12px}
.ask{display:flex;flex-wrap:wrap;align-items:center;gap:14px}.ask p{font-weight:600}
.btn{display:inline-flex;align-items:center;justify-content:center;min-height:44px;padding:0 18px;border:1px solid var(--edge);border-radius:8px;background:transparent;color:var(--text);font:inherit;text-decoration:none;white-space:nowrap;cursor:pointer}
.btn:hover{border-color:var(--quiet);color:var(--text)}
.btn.primary{border-color:transparent;background:var(--accent);color:var(--on-accent);font-weight:600;padding:0 20px}.btn.primary:hover{background:var(--accent-hi);color:var(--on-accent)}
.btn.wide{width:100%;min-height:48px}
.end{padding-top:32px;display:flex;justify-content:flex-end}
.made{display:flex;align-items:center;gap:8px;min-height:44px;color:var(--quiet);font-size:13px;text-decoration:none}.made:hover{color:var(--text)}.made svg{display:block;width:17px;height:18px}
@media (max-width:600px){.join{padding-top:24px;gap:24px}#get{gap:24px}#get>.line:first-child{margin-top:-12px}}`;

// ───────── the tab icon (DECISIONS B109) ─────────

/** Every page's head names its icon the way the landing does (site/index.html): the SVG,
 * a PNG for browsers that draw no SVG icon in a tab (Safari), and the same PNG for a
 * phone's home screen. Always there — the tab icon isn't the mark and doesn't follow its
 * switch: without one a browser draws a blank. */
const TAB_ICON_LINKS = `<link rel="icon" type="image/svg+xml" href="/favicon.svg"><link rel="icon" type="image/png" sizes="180x180" href="/favicon.png"><link rel="apple-touch-icon" href="/favicon.png">`;

/** The icon's three addresses. The art rides in this file, so an icon costs no storage
 * read, needs no cookie, and is there the moment a space's page is — nothing to place in
 * the bucket. `/favicon.ico` is the one browsers ask for unprompted (a tab on a poster or
 * a download, the embed face opened on its own): it answers with the PNG, which a browser
 * reads by its bytes. Kept for a year — the art changes only with the landing's, and a
 * page that must show a new one at once can name it by a new address (`?v=2`). */
/** @param {Request} request @param {string} path */
function tabIcon(request, path) {
  const svg = path === "/favicon.svg";
  const bytes = svg ? new TextEncoder().encode(TAB_ICON_SVG) : tabIconPng();
  const headers = { "content-type": svg ? "image/svg+xml" : "image/png", "content-length": String(bytes.length), "cache-control": "public, max-age=31536000, immutable", "x-robots-tag": "noindex" };
  return new Response(request.method === "HEAD" ? null : bytes, { status: 200, headers });
}

/** @type {Uint8Array<ArrayBuffer> | null} */
let tabIconBytes = null;
/** The PNG, decoded once per isolate. */
function tabIconPng() {
  if (!tabIconBytes) tabIconBytes = Uint8Array.from(atob(TAB_ICON_PNG.replace(/\s/g, "")), (c) => c.charCodeAt(0));
  return tabIconBytes;
}

/** The landing's art, byte for byte: site/favicon.svg as text, site/favicon.png (180×180)
 * as base64, wrapped at any width — its line breaks are dropped before decoding.
 * test/icon.test.js holds both equal to the landing's files, so the two can't drift:
 * change the art there, then here. */
const TAB_ICON_SVG = `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 64 64" role="img" aria-label="shroom">
  <rect width="64" height="64" rx="14" fill="#2E3440"/>
  <g transform="translate(-1.85,-4.66) scale(0.564)">
    <path d="M50,68 C49,82 46,92 46,96 C46,101 52,104 60,104 C68,104 74,101 74,96 C74,92 71,82 70,68 Z" fill="#E5E9F0"/>
    <path d="M24,64 C24,42 40,26 60,26 C80,26 96,42 96,64 C86,75 34,75 24,64 Z" fill="#88C0D0"/>
    <circle cx="33" cy="58" r="3.4" fill="#ECEFF4"/>
    <circle cx="88" cy="58" r="2.2" fill="#ECEFF4"/>
    <circle cx="50" cy="44" r="6" fill="#ECEFF4"/>
    <circle cx="72" cy="48" r="3.6" fill="#ECEFF4"/>
    <circle cx="64" cy="58" r="2.6" fill="#ECEFF4"/>
  </g>
</svg>
`;

const TAB_ICON_PNG = `
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
`;
