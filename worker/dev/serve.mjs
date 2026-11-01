// A local stand-in for a space's Worker, for the app's dev drives (never deployed, never
// shipped): the real `worker.js` — its exported `handle(request, env, now)` — served over
// plain HTTP on 127.0.0.1, one in-memory bucket per space (the unit tests' fake bucket),
// each seeded as wiring leaves a space: the cookie key, a fresh roster with member #1
// already in it (registry::seed), and an `R2_PARENT` binding.
//
//   node worker/dev/serve.mjs [--port N] [--space <host>[=<owner name>]]…
//
// Prints one line, `{"url": "http://127.0.0.1:<port>", "spaces": [...]}`, then serves until
// its stdin closes. A dev build reaches it with `--space-base <url>` (loopback only); the
// app then calls `<url>/<host>/<path>` where a release build calls `https://<host>/<path>`.
//
// Routes:
//   /<host>/<path>                the Worker, as `https://<host>/<path>`
//   /_s3/<host>/<bucket>/<key>    the space's bucket over a small S3 surface (PUT with
//                                 If-None-Match / If-Match, GET, HEAD, DELETE, ListObjectsV2)
//                                 — where a minted credential's uploads land. Every request
//                                 must carry a credential the space's parent stands behind:
//                                 either a minted one — the session token's JWT checked
//                                 against the parent (HS256, `exp`, `bucket`, `iss`, the
//                                 paths) and the SigV4 signature against its secret (SHA-256
//                                 of the JWT) — or the parent pair itself (a token Mac: its
//                                 ledger entries, Library, owner grant), as R2 takes it.
//   /_dev/space/<host>/device     POST {id, verifier, member, label?, name?, token?}: a Mac
//                                 already in the space (a drive's seeded home)
//   /_dev/space/<host>/invite     POST {id, codeHash, kind, label?, inviter?, hours?}: a
//                                 pending invite (only the code's hash, as the app sends it)
//   /_dev/space/<host>/parent     POST {account_id, bucket, key_id, secret}: the space's
//                                 `R2_PARENT` (a token Mac's drive: the token's derived pair;
//                                 the L1 fake's `--relay-parent` posts here when the app
//                                 places the secret — DECISIONS B90). GET: the binding as
//                                 {account_id, bucket, key_id, placed} — never its secret;
//                                 `placed` counts the POSTs since start. Until the first
//                                 POST the binding is the space's own derived pair: another
//                                 token's, i.e. stale for any Mac's token (the stale drive)
//   /_dev/space/<host>/forget     POST {id}: the space loses a device's record (an enroll
//                                 that never landed, a restored bucket — the token drive)
//   /_dev/space/<host>/fault      POST {prefix, status, count?}: the space's Worker answers
//                                 `status` (no body work) to the next `count` requests whose
//                                 path starts with `prefix` (no count: until cleared) — the
//                                 L6 drive's busy space; POST {clear: true} clears them
//   /_dev/space/<host>/put        POST {key, json}: a JSON object under meta/, rec/ or
//                                 .shroom/views/ — a video and its counts as Macs and
//                                 viewers left them (the cleanup drive, N68)
//   /_dev/space/<host>/schedule   POST {aheadHours?, runs?}: the page's own schedule (N75's
//                                 auto-cleanup, `autocleanTick`) run `runs` times (default:
//                                 until its walk has read every record, then once more), at
//                                 now plus `aheadHours` — the seam's clock, so a drive sees the
//                                 page warn and then delete, days apart, in a minute. Answers
//                                 the auto-cleanup state the runs left
//   /_dev/space/<host>/roster     GET: the roster as the Worker keeps it
//   /_dev/space/<host>/objects    GET: every key in the bucket, with its size and metadata
//                                 (and the text of the small JSON ones: records, ledger,
//                                 plays and the plays switch)
//
// Choice (documented for the drives): uploads go to this server's own bucket, not the L1
// Cloudflare fake's — the ledger the Worker writes at `/mint`, the record and the bytes all
// land in one place the drive can read back.
import { createHash, createHmac, timingSafeEqual } from "node:crypto";
import { createServer } from "node:http";
import { autocleanTick, handle } from "../src/worker.js";
import { space, COOKIE_KEY } from "../test/space-fixture.js";

const args = process.argv.slice(2);
const flag = (name) => { const i = args.indexOf(name); return i >= 0 ? args[i + 1] : undefined; };
const port = Number(flag("--port") ?? 0);
const wanted = [];
args.forEach((a, i) => { if (a === "--space" && args[i + 1]) wanted.push(args[i + 1]); });
if (wanted.length === 0) wanted.push("v.myshroom.workers.dev=Bagrat");

const ROSTER = ".shroom/registry/roster.json";
const sha = (s) => createHash("sha256").update(s).digest("hex");
const iso = (ms) => new Date(ms).toISOString().replace(/\.\d{3}Z$/, "Z");

/** @type {Map<string, {env: any, parent: any, owner: string, placed?: number, faults?: {prefix: string, status: number, left: number | null}[]}>} */
const spaces = new Map();
for (const w of wanted) {
  const [host, owner = "Bagrat"] = w.split("=");
  // A parent per space, derived from its host: two spaces never share one.
  const parent = { account_id: sha(`account:${host}`).slice(0, 32), bucket: "shroom-space", key_id: sha(`key:${host}`).slice(0, 32), secret: sha(`secret:${host}`) };
  const at = iso(Date.now());
  const roster = { v: 1, createdAt: at, members: { m1: { role: "owner", name: owner, state: "active", joinedAt: at } }, devices: {}, invites: {} };
  spaces.set(host, { env: space({ roster, parent }), parent, owner });
}
void COOKIE_KEY;

async function bodyOf(req) {
  const chunks = [];
  for await (const c of req) chunks.push(c);
  return Buffer.concat(chunks);
}

function send(res, status, body = "", headers = {}) {
  const bytes = typeof body === "string" ? Buffer.from(body) : Buffer.from(body);
  res.writeHead(status, { "content-length": String(bytes.length), ...headers });
  res.end(bytes);
}

const json = (res, status, v) => send(res, status, JSON.stringify(v), { "content-type": "application/json" });
const xmlError = (res, status, code) => send(res, status, `<Error><Code>${code}</Code></Error>`, { "content-type": "application/xml" });
const esc = (s) => s.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;");

async function readRoster(env) {
  const o = await env.BUCKET.get(ROSTER);
  return JSON.parse(await o.text());
}

async function writeRoster(env, roster) {
  await env.BUCKET.put(ROSTER, JSON.stringify(roster));
}

// ───────── the Worker ─────────

async function worker(req, res, host, rest, search) {
  const s = spaces.get(host);
  if (!s) return send(res, 404, "error code: 1042"); // workers.dev's answer for no Worker (B35 P7)
  const f = (s.faults ?? []).find((x) => `/${rest}`.startsWith(x.prefix) && x.left !== 0);
  if (f) {
    if (f.left !== null) f.left--;
    await bodyOf(req);
    process.stderr.write(`serve: fault ${f.status} ${req.method} ${host}/${rest}\n`);
    return json(res, f.status, { shroom: 1, error: "busy" });
  }
  const headers = new Headers();
  for (const [k, v] of Object.entries(req.headers)) {
    if (["host", "connection", "content-length", "transfer-encoding"].includes(k)) continue;
    headers.set(k, Array.isArray(v) ? v.join(", ") : v);
  }
  const init = { method: req.method, headers };
  if (req.method !== "GET" && req.method !== "HEAD") init.body = await bodyOf(req);
  const out = await handle(new Request(`https://${host}/${rest}${search}`, init), s.env, Date.now());
  const h = {};
  out.headers.forEach((v, k) => { h[k] = v; });
  const bytes = Buffer.from(await out.arrayBuffer());
  delete h["content-length"];
  send(res, out.status, req.method === "HEAD" ? "" : bytes, h);
}

// ───────── the bucket over S3 ─────────

const enc = (s) => [...Buffer.from(s, "utf8")].map((b) => (/[A-Za-z0-9\-_.~]/.test(String.fromCharCode(b)) ? String.fromCharCode(b) : `%${b.toString(16).toUpperCase().padStart(2, "0")}`)).join("");
const dec = (s) => { try { return decodeURIComponent(s); } catch { return s; } };
const hmac = (key, data) => createHmac("sha256", key).update(data).digest();

/** The credential a request carries, checked against the space's parent: the JWT inside
 * its session token, and the SigV4 signature under the JWT's derived secret. */
function authorize(req, s, rawPath, query, body, bucket, key, write) {
  const token = req.headers["x-amz-security-token"];
  const auth = req.headers["authorization"] ?? "";
  let secret;
  if (token === undefined) {
    // The parent pair itself (the token's derived keys): the whole bucket, as on R2.
    secret = s.parent.secret;
  } else {
    if (typeof token !== "string") return "a malformed session token";
    const raw = Buffer.from(token, "base64").toString("utf8");
    if (!raw.startsWith("jwt/")) return "not a minted credential";
    const jwt = raw.slice(4);
    const [h, p, sig] = jwt.split(".");
    if (!h || !p || !sig) return "malformed token";
    const want = createHmac("sha256", Buffer.from(s.parent.secret, "utf8")).update(`${h}.${p}`).digest();
    const got = Buffer.from(sig, "base64url");
    if (got.length !== want.length || !timingSafeEqual(got, want)) return "the token isn't signed by this space's parent";
    const c = JSON.parse(Buffer.from(p, "base64url").toString("utf8"));
    if (c.exp * 1000 < Date.now()) return "expired";
    if (c.bucket !== bucket || c.iss !== s.parent.key_id || c.sub !== s.parent.account_id) return "another bucket or parent";
    if (write && c.scope !== "object-read-write") return "read-only";
    if (c.paths) {
      const prefixes = c.paths.prefixPaths ?? [];
      const objects = c.paths.objectPaths ?? [];
      if (!(prefixes.some((x) => key.startsWith(x)) || objects.includes(key))) return `outside the credential's paths: ${key}`;
    }
    secret = createHash("sha256").update(jwt).digest("hex");
  }
  // SigV4 (S3): the canonical request as the client built it, under the credential's secret.
  const m = auth.match(/^AWS4-HMAC-SHA256 Credential=([^/]+)\/(\d{8})\/([^/]+)\/s3\/aws4_request, SignedHeaders=([^,]+), Signature=([0-9a-f]{64})$/);
  if (!m) return "no SigV4 signature";
  const [, akid, date, region, signedHeaders, signature] = m;
  if (akid !== s.parent.key_id) return "another access key";
  const payload = createHash("sha256").update(body).digest("hex");
  if (req.headers["x-amz-content-sha256"] !== payload) return "the body isn't the one signed";
  const uri = rawPath.split("/").map((seg) => enc(dec(seg))).join("/");
  const q = [...query.entries()].map(([k, v]) => [enc(k), enc(v)]).sort((a, b) => (a[0] < b[0] ? -1 : a[0] > b[0] ? 1 : a[1] < b[1] ? -1 : 1)).map(([k, v]) => `${k}=${v}`).join("&");
  const names = signedHeaders.split(";");
  const canonHeaders = names.map((n) => `${n}:${String(req.headers[n] ?? "").trim()}\n`).join("");
  const canonical = [req.method, uri, q, canonHeaders, signedHeaders, payload].join("\n");
  const amzDate = req.headers["x-amz-date"];
  const scope = `${date}/${region}/s3/aws4_request`;
  const toSign = ["AWS4-HMAC-SHA256", amzDate, scope, createHash("sha256").update(canonical).digest("hex")].join("\n");
  const kSigning = hmac(hmac(hmac(hmac(`AWS4${secret}`, date), region), "s3"), "aws4_request");
  if (hmac(kSigning, toSign).toString("hex") !== signature) return "SignatureDoesNotMatch";
  return null;
}

async function s3(req, res, host, rest, rawPath, query) {
  const s = spaces.get(host);
  if (!s) return xmlError(res, 404, "NoSuchBucket");
  const slash = rest.indexOf("/");
  const bucket = dec(slash < 0 ? rest : rest.slice(0, slash));
  const key = slash < 0 ? "" : dec(rest.slice(slash + 1));
  if (bucket !== s.parent.bucket) return xmlError(res, 404, "NoSuchBucket");
  const body = await bodyOf(req);
  const B = s.env.BUCKET;
  const listing = key === "" && req.method === "GET";
  const why = authorize(req, s, rawPath, query, body, bucket, listing ? (query.get("prefix") ?? "") : key, !["GET", "HEAD"].includes(req.method));
  if (why) {
    process.stderr.write(`serve: s3 ${req.method} ${host} refused: ${why}\n`);
    return xmlError(res, 403, "AccessDenied");
  }
  if (listing) {
    const prefix = query.get("prefix") ?? "";
    let xml = "<ListBucketResult><IsTruncated>false</IsTruncated>";
    let cursor;
    do {
      const page = await B.list({ prefix, cursor });
      for (const o of page.objects) xml += `<Contents><Key>${esc(o.key)}</Key><Size>${o.size}</Size><ETag>"${o.etag}"</ETag></Contents>`;
      cursor = page.truncated ? page.cursor : undefined;
    } while (cursor);
    return send(res, 200, xml + "</ListBucketResult>", { "content-type": "application/xml" });
  }
  switch (req.method) {
    case "PUT": {
      const cond = new Headers();
      if (req.headers["if-none-match"]) cond.set("if-none-match", req.headers["if-none-match"]);
      if (req.headers["if-match"]) cond.set("if-match", req.headers["if-match"]);
      const customMetadata = {};
      for (const [k, v] of Object.entries(req.headers)) if (k.startsWith("x-amz-meta-")) customMetadata[k.slice(11)] = String(v);
      const put = await B.put(key, new Uint8Array(body), { onlyIf: [...cond.keys()].length ? cond : undefined, customMetadata: Object.keys(customMetadata).length ? customMetadata : undefined });
      if (!put) return xmlError(res, 412, "PreconditionFailed");
      return send(res, 200, "", { etag: `"${put.etag}"` });
    }
    case "GET":
    case "HEAD": {
      const o = await B.get(key);
      if (!o) return xmlError(res, 404, "NoSuchKey");
      const bytes = Buffer.from(await new Response(o.body).arrayBuffer());
      return send(res, 200, req.method === "HEAD" ? "" : bytes, { etag: `"${o.etag}"`, "content-type": "application/octet-stream" });
    }
    case "DELETE":
      await B.delete(key);
      return send(res, 204);
    default:
      return xmlError(res, 405, "MethodNotAllowed");
  }
}

// ───────── the drives' knobs ─────────

async function dev(req, res, host, what) {
  const s = spaces.get(host);
  if (!s) return json(res, 404, { error: "no such space" });
  const B = s.env.BUCKET;
  const now = iso(Date.now());
  if (req.method === "GET" && what === "roster") return json(res, 200, await readRoster(s.env));
  if (req.method === "GET" && what === "parent") {
    return json(res, 200, { account_id: s.parent.account_id, bucket: s.parent.bucket, key_id: s.parent.key_id, placed: s.placed ?? 0 });
  }
  if (req.method === "GET" && what === "objects") {
    const page = await B.list({ include: ["customMetadata"] });
    const out = [];
    for (const o of page.objects) {
      const small = o.key.startsWith("meta/") || o.key.startsWith(".shroom/registry/recs/") || o.key.startsWith(".shroom/views/") || o.key === ".shroom/plays.json" || o.key.startsWith(".shroom/autoclean");
      const text = small ? await (await B.get(o.key)).text() : undefined;
      out.push({ key: o.key, size: o.size, customMetadata: o.customMetadata ?? {}, ...(text === undefined ? {} : { text }) });
    }
    return json(res, 200, out);
  }
  if (req.method !== "POST") return json(res, 405, { error: "method" });
  const b = JSON.parse((await bodyOf(req)).toString("utf8") || "{}");
  if (what === "fault") {
    if (b.clear) { s.faults = []; return json(res, 200, { ok: true }); }
    if (typeof b.prefix !== "string" || !Number.isInteger(b.status)) return json(res, 400, { error: "a fault has a prefix and a status" });
    (s.faults ??= []).push({ prefix: b.prefix, status: b.status, left: Number.isInteger(b.count) ? b.count : null });
    return json(res, 200, { ok: true });
  }
  if (what === "put") {
    // A JSON object as a Mac or a viewer's plays left it (the cleanup drive's videos and counts).
    if (typeof b.key !== "string" || !/^(meta\/|rec\/|\.shroom\/views\/)/.test(b.key) || b.json === undefined) return json(res, 400, { error: "put takes a meta/, rec/ or .shroom/views/ key and json" });
    await B.put(b.key, JSON.stringify(b.json), { httpMetadata: { contentType: "application/json" } });
    return json(res, 200, { ok: true });
  }
  if (what === "schedule") {
    // The cron's run, as the page's `scheduled` makes it — at the drive's hour.
    const at = Date.now() + (Number(b.aheadHours) || 0) * 3600 * 1000;
    const state = async () => { const o = await B.get(".shroom/autoclean/state.json"); return o ? JSON.parse(await o.text()) : null; };
    let runs = 0;
    if (Number.isInteger(b.runs)) {
      for (; runs < b.runs; runs++) await autocleanTick(s.env, at);
    } else {
      for (; runs < 60; runs++) {
        await autocleanTick(s.env, at);
        const st = await state();
        if (!st || (st.cursor === null && st.scanAt > 0)) break;
      }
      await autocleanTick(s.env, at);
      runs++;
    }
    return json(res, 200, { at: iso(at), runs, state: await state() });
  }
  if (what === "parent") {
    if (!["account_id", "bucket", "key_id", "secret"].every((k) => typeof b[k] === "string" && b[k])) return json(res, 400, { error: "a parent has account_id, bucket, key_id and secret" });
    s.parent = { account_id: b.account_id, bucket: b.bucket, key_id: b.key_id, secret: b.secret };
    s.env.R2_PARENT = JSON.stringify(s.parent);
    s.placed = (s.placed ?? 0) + 1;
    process.stderr.write(`serve: R2_PARENT placed for ${host} (key ${b.key_id.slice(0, 6)}…)\n`);
    return json(res, 200, { ok: true });
  }
  const r = await readRoster(s.env);
  if (what === "device") {
    if (b.member !== "m1" && !r.members[b.member]) {
      r.members[b.member] = { role: "member", label: b.label ?? "Member", inviter: s.owner, state: "active", joinedAt: now, endedAt: null, invite: null, reinvites: [] };
    }
    r.devices[b.id] = {
      member: b.member, name: b.name ?? null, verifier: b.verifier, state: "active", joinedAt: now, endedAt: null,
      via: b.token ? "enroll" : "dev-seed", tokenKey: b.token ? s.parent.key_id : null, proofAt: b.token ? now : null,
    };
  } else if (what === "forget") {
    if (!r.devices[b.id]) return json(res, 404, { error: "no such device" });
    delete r.devices[b.id];
  } else if (what === "invite") {
    const hours = b.hours ?? (b.kind === "own_mac" ? 1 : 24 * 7);
    r.invites[b.id] = {
      codeHash: b.codeHash, kind: b.kind ?? "member", label: b.label ?? null, inviter: b.inviter ?? s.owner, by: null,
      createdAt: now, expiresAt: iso(Date.now() + hours * 3600 * 1000), state: "pending", used: null, endedAt: null,
    };
  } else {
    return json(res, 404, { error: "no such knob" });
  }
  await writeRoster(s.env, r);
  return json(res, 200, { ok: true });
}

const server = createServer(async (req, res) => {
  try {
    const u = new URL(req.url, "http://127.0.0.1");
    const parts = u.pathname.split("/").slice(1);
    if (parts[0] === "_s3" && parts.length >= 3) {
      return await s3(req, res, parts[1], parts.slice(2).join("/"), u.pathname, u.searchParams);
    }
    if (parts[0] === "_dev" && parts[1] === "space" && parts.length === 4) {
      return await dev(req, res, parts[2], parts[3]);
    }
    if (parts[0] && parts[0].includes(".")) {
      return await worker(req, res, parts[0], parts.slice(1).join("/"), u.search);
    }
    send(res, 404, "error code: 1042");
  } catch (e) {
    process.stderr.write(`serve: ${e?.stack ?? e}\n`);
    if (!res.headersSent) send(res, 500, "");
  }
});

server.listen(port, "127.0.0.1", () => {
  const { port: p } = /** @type {import("node:net").AddressInfo} */ (server.address());
  process.stdout.write(`${JSON.stringify({ url: `http://127.0.0.1:${p}`, spaces: [...spaces.keys()] })}\n`);
});
process.stdin.on("end", () => process.exit(0));
process.stdin.resume();
process.on("SIGTERM", () => process.exit(0));
