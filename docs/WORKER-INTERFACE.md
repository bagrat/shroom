# shroom v1 — the Worker interface

> **Status.** The Worker interface as built (2026-10-14, page VERSION/tree 0.1.9). The
> design record this replaced is [`history/design/WORKER-INTERFACE.md`](../history/design/WORKER-INTERFACE.md); decisions are
> [`DECISIONS.md`](DECISIONS.md). N29–N42 are designed, not ratified, so this contract is provisional; §9
> lists what is still Bagrat's. Where this file and the code disagree, the code wins.
>
> **Citations.** `D:Bxx` = a DECISIONS.md row (a trailing letter = its lettered item).
> `DV #n` = a deviation (DECISIONS.md, cited DEVIATIONS #n). `Nxx` = a spec row; `SM:<line>` = a line of the retired spec text, history/spec/UX-STATEMACHINE.md.
> Code = `path` or `path:function`. Tags like H1, C5, U7 (the design review's findings) and
> P1–P9 (live probes, §7.2) stay only because code comments cite them; their reasoning is
> in the history copy.

---

## 1. Purpose and standing

### 1.1 What it covers

Every route, credential, stored record and on-disk file shared by the Worker
(`worker/src/worker.js`) and the Mac (`crates/shroom-cloud`, `app/src-tauri/src/runtime`):
members, invites, the join machine, member life (SM:1282-1590) and the page. Built by
D:B37 (Worker) and D:B39–B42 (core, app). Later changes: D:B38, B49–B54, B91, B94,
B108–B110, B125.

### 1.2 What stays Bagrat's

§9. Building a call doesn't ratify it.

### 1.3 Constraints the build holds to

- One dependency-free ESM file, WebCrypto only, `@ts-check` strict; deployed as is.
- Every write is idempotent: callers send ids, or hashes of secrets, they made.
- Every authenticated request reads the roster fresh, so a cancel or removal bites on the
  next request (SM:1366, 1299-1302).
- No secret in argv, logs or URLs, except law 13's join link in an install command
  (SM:153-155); see §6.9 and §6.12.
- Members never hold the token, the parent pair, or a credential over `.shroom/`.

---

## 2. The registry

### 2.1 The store: R2, one document

The **roster** is `.shroom/registry/roster.json` in the private bucket (`shroom-space`,
D:B24). It holds members, devices and invites. Every change is one compare-and-swap
(CAS): read, apply a pure function, write back with `onlyIf:{etagMatches}`. So consume is
atomic and single-use, cancel and remove take effect at once, and retries are safe.

P1 proved one winner per conditional put; the loser gets `null` (D:B35). Wiring seeds the
roster over S3 (§6.11), so it survives losing the Worker (SM:651).

Only the registry code reads `.shroom/registry/`. No recording credential covers
`.shroom/` (§3.4, §3.6). The roster holds hashes only; labels are admin-private
plaintext.

### 2.2 The narrow interface

No other `worker.js` code touches registry keys.

| Function | Contract |
|---|---|
| `readRoster` | One fresh read. Absent → 503 `space`/`unwired`. Not JSON, or `members`/`devices`/`invites` not objects → `space`/`broken` (left as found). Binding error → `space`/`storage`. **Never creates or repairs it.** |
| `mutate(env, now, fn, first?)` | ≤ 5 tries (`CAS_TRIES`), 50–400 ms jitter between. Each: read (try 1 may reuse the auth read), `prune`, `fn` → `{write?, result}` or a thrown `Refusal`; then `casPut(…, {etagMatches})` if writing. 5 losses → 503 `busy`. |
| `casPut` | `true` if written. A `null` **or a throw** is `false`, so a rate refusal (C5) is a retry too. |
| `ledgerGet` | `.shroom/registry/recs/<id>`: its JSON if `member` is a string, else `customMetadata.m`, else `null`. |
| `ledgerCreate` | Create-only (`etagDoesNotMatch:"*"`, `customMetadata {m}`). On conflict, the existing entry. 3 failed rounds → 503 `busy`. |

**Mac side** (`shroom-cloud/src/registry.rs`):
- `seed`: a create-only S3 PUT of `initial()`, for wiring, the page refresh and enroll's
  `unwired` path.
- `is_roster`: `readRoster`'s shape test.
- `set_aside`: copies an unparseable roster create-only to
  `.shroom/registry/roster.broken-<unix>.json` (never deletes it). **No app caller yet**
  (§6.11).

### 2.3 The roster schema (v1; additive only; readers ignore unknown fields)

```jsonc
{ "v": 1, "createdAt": "iso",
  "members": {
    "m1": { "role": "owner", "name": "Bagrat" | null, "state": "active", "joinedAt": "iso" },
    "m_<16>": { "role": "member", "label", "inviter", "state": "active" | "removed" | "left",
                "joinedAt", "endedAt": null | "iso", "invite": "i_…",
                "reinvites": [ { "invite", "label", "at", "joinedBefore" } ] } },
  "devices": {
    "d_<16>": { "member": "m1" | "m_…", "name": "…" | null, "verifier": "<64 hex>",
                "state": "active" | "removed" | "left", "joinedAt", "endedAt": null | "iso",
                "via": "enroll" | "i_…", "tokenKey": "<key_id>" | null, "proofAt": "iso" | null,
                "machine"?: "<64 hex>" } },
  "invites": {
    "i_<16>": { "codeHash": "<64 hex>", "kind": "member" | "own_mac", "label": "…" | null,
                "inviter", "by": "d_…", "createdAt", "expiresAt", "state": "pending" | "used" | "cancelled",
                "used": null | { "at", "member", "device", "verifier" }, "endedAt": null | "iso" } } }
```

- `verifier` / `codeHash` = lower-case hex SHA-256 of the device secret / the code.
- A device `name` is kept only for own-Mac joins and enrolls.
- `tokenKey` is informational; the token flag reads `proofAt` (§3.2).
- The seed (`registry::initial`) already holds member #1:
  `members:{m1:{role:"owner", name:null, state:"active", joinedAt}}`. The first enroll with
  `ownerName` fills the name.

**Ids** are told apart by length and place, never prefix (C3). `m1` is literal.
`m_`/`d_`/`i_` + 16 base64url (`MEMBER_ID`/`DEVICE_ID`/`INVITE_ID`) are made by the Worker
at join, the Mac before its first call, and the inviting Mac. Recording ids and join codes
are `ID` = `^[A-Za-z0-9_-]{16}$` (B14).

**Expired** = pending with `now > expiresAt` (`inviteState`), never stored. Lifetimes are
server-set (`INVITE_SECONDS`): 604800 s for member invites (SM:1328), 3600 s for own-Mac
(SM:1387).

**Pruning** (`prune`, every `mutate`):
- ended devices: 180 days after `endedAt`;
- used invites: once their device is gone;
- cancelled invites: 30 days after `endedAt`;
- pending own-Mac invites: 30 days after `expiresAt`;
- pending member invites and members: never.

**Caps** (`CAPS`):

| Cap | Limit and answer |
|---|---|
| unexpired pending invites | 20 → 409 `full` |
| invites in the document | 500. `evictInvites` first (oldest first: cancelled, expired own-Mac, used behind a non-active device; D:B38), then 409 `full` |
| active members | 200 → 409 `full` at join |
| active `m1` devices | 20 → 409 `full` at an own-Mac join. Enroll is exempt (D:B38) |
| `label` / `inviter`, `ownerName` | 64 / 40 characters → 400 |
| Mac `name` | cut to 64, never refused |

Text is cleaned by stripping `\p{Cc}`, then NFC, then trim (`typed`, `macName`).

**Budgets live outside the roster:** `.shroom/registry/rate/<did>` =
`{"window":"<iso hour>","n":<uploads>,"k":<page keys>}`, written by CAS or create-only
(`spendBudget`, §3.4).

### 2.4 The ownership ledger (§16 #11)

`.shroom/registry/recs/<id>` = `{"v":1,"member":"m_…"|"m1","device":"d_…"|null,"at":"iso"}`,
with `customMetadata {m}` (over S3: `x-amz-meta-m`).

**Written** create-only, once per recording, before its first byte:
- by the Worker at an id's first `upload` mint (§4.8);
- by a token Mac over S3 (`put_if_absent_with_meta`) before it mints locally
  (`runtime/uploader.rs:mint_locally`). `device` is `null` with no device key. A refused
  PUT (401/403/404) blocks the take as space-side.

Nothing rewrites or deletes it; the sweep never touches it (D:B91).

**Owner of a recording** (`ownerOfRecording`, `mint`, `gone`, `library`):
1. the ledger's member;
2. else `m1`, if a meta or a deletion record (§4.20) exists (pre-ledger spaces);
3. else nobody: whoever mints first.

`meta.owner` is written by the Mac from mint's `owner`, for readability only; nothing
trusts it. *Mine · Everyone*, row labels and "This is Mher's video." key on the ledger
(D:B54 f).

**Residual.** A capture whose start and Stop were both offline has a link before its
ledger entry. Whoever mints first owns it; the maker then reads 403 `not-yours` and keeps
the take.

### 2.5 Delta if Bagrat picks a Durable Object

Not built; P1 passed (D:B35). See history/design/WORKER-INTERFACE.md §2.5.

---

## 3. Auth and credentials

### 3.1 The ways in

| Credential | Sent as | Proves | Revoked by |
|---|---|---|---|
| **Device key** (every Mac) | `Authorization: Bearer <did>.<secret>` (secret 16–128 base64url) | the device and its member are active | Remove…, Leave, Disconnect |
| **Parent proof** (token Macs) | `Authorization: ShroomParent <ts>.<sig>` | knowing the current `R2_PARENT` secret | re-placing `R2_PARENT` (§3.4) |
| **Join code** | the `/join/<code>` path; peek and join bodies | the right to consume one pending invite | Cancel, use, expiry |
| **Owner cookie** (a browser) | `shroom_owner` | the issuing device is active (metadata powers, and one picture per title beside what it may rename, D:B164) | removing that device or its member |
| **`k`** | `?k=` on `/<id>` | one redeem of a grant | the redeem, or 600 s |
| **Recording credential** | S3 SigV4 + session token, direct to R2 | R2 enforces `rec/<id>/` + `meta/<id>.json` | expiry: 7 days upload, 15 min manage |

No credential is needed for the public pages, `/m/`, `/d/`, status, `/oembed`, ping, the
icon, or `/join/<code>` (which reads a cookie only to pick a face).

**The `shroom` command** (spec N62, D:B215) calls these routes under its Mac's device key,
exactly as that Mac's app does — `library`, `mint` (a `manage` credential for a rename, a
delete or an edit's save, spec N72), `gone`, `members`, `invites` and their deletes — so the page holds it to the same
role. It never holds the parent proof or the token. Where a page makes cleanup suggestions
(`POST /_shroom/api/cleanup`, the cleanup proposal's route) it reads them; a page that answers
404 there gets age alone.

### 3.2 Device key and parent proof

**Device key.** The Mac makes `did` (`d_` + 12 bytes) and `secret` (32 bytes, 43 characters)
before its first call, in a file (`device.key`, or `join.json`), so retries resend the
same values. Join and enroll send only `verifier`; later calls send the secret as the
bearer. `DeviceKey`, `JoinFile` and `CredFile` print redacted; the Worker logs nothing.

**The check** (`bearer` + `caller`, after the roster read):

| Condition | Answer |
|---|---|
| malformed header, unknown `did`, wrong secret, or the member absent | 401 `unauthorized` |
| the device isn't active, or its member isn't | 403 `removed`, `why: "left"\|"removed"` |

Roles are read live. Owner-class = `members[m].role == "owner"`, which only `m1` has.

**Parent proof** (`member::parent_proof`, checked in `worker.js:enroll`):

```
sig = b64url(HMAC-SHA256(UTF-8 of R2_PARENT.secret,
      "shroom-parent-v1\n" + METHOD + "\n" + host + "\n" + path + "\n" + ts + "\n" + hex(sha256(body))))
```

- `ts` is unix seconds.
- The key is the token's derived S3 secret, the one `mint.rs` signs JWTs with.
- The prefix keeps any JWT signing input from passing as a proof.
- A replay inside the window only repeats an idempotent enroll.

| Answer | When |
|---|---|
| 401 `unauthorized` | the header is missing or malformed |
| 503 `space`/`no-parent` | `R2_PARENT` missing or unparseable |
| 401 `clock` | the clock is more than 300 s off |
| 401 `stale-parent` | the HMAC fails |

A proof can only:
- create or revive a device **under `m1`**, including one another owner Mac removed (U8);
- stamp `tokenKey` and `proofAt`;
- create `m1` or refresh its `name`.

**Token-holder flag** (`tokenHolder`): an active `m1` device with `proofAt` ≤ 30 days old.

**Restamp throttle (C5).** Enroll writes only when one of these holds:
- `m1` is absent, or `ownerName` differs from its name;
- the device is new or not active;
- its `name` changed;
- `proofAt` is missing or more than 24 h old.

Otherwise it answers 200 without writing.

### 3.3 Join code and `k`

**Join code.**
- 12 CSPRNG bytes = 16 base64url characters (96 bits).
- The inviter sends only `codeHash`; the Worker sees the plaintext only at peek, join and
  the join page, and never stores it.
- It rides only the `/join/` path and POST bodies (the page is `no-referrer`).
- Single use comes from the CAS. There are no rate counters.

**`k`.**
- 32 bytes, 43 characters, made inside the open and never logged.
- `/key` (§4.9) stores `.shroom/keys/<sha256(k)>` = `{member, device, exp: now+600 s}`
  create-only (`KEY_SECONDS` = `owner::KEY_TTL_SECS`).
- The redeem (§4.17) spends it by CAS (`{…, spent}`); only the winner gets a cookie, and
  the spent grant is deleted after the reply.

**Fallback (C2).** When `/key` isn't shroom-shaped (a page before iface 2), a token Mac
writes the grant over S3: `{member:"m1", role:"owner", device, exp}`
(`owner::grant_for_device`), or with no device key B25's `{member, role, exp}`
(`owner::grant`), for which an iface-2 page issues no cookie. Token-less Macs open the
page clean (`runtime/cloud.rs:owner_key`).

**Reaping.** The sweep reaps grants that are spent, more than 1 h past `exp`, or
unparseable (`library::key_done`, D:B91).

### 3.4 Recording credentials: the Worker's mint

**The binding.** `R2_PARENT` (`secret_text`) = `{"account_id","bucket","key_id","secret"}`,
the token's derived S3 pair (`wire::parent_text`). `parentOf` needs all four non-empty,
else it reads as missing.

**Placement** (token Macs only; H1):
1. every script PUT carries it (`wire::put_page`);
2. a rewire over a page at this `VERSION` or newer re-places it alone (`wire::place_parent`,
   the secrets endpoint, P5) — space-found's re-place;
3. an enroll answered `stale-parent`/`no-parent` on a Mac whose token the sweep just proved
   live re-places it once, then re-enrolls every 3 s for ≤ 60 s. Still stale →
   `EnrollFail::StaleAfterReplace`, never a loop.

With two live tokens, the parent follows the Mac that entered last.

**The mint** (`worker.js:memberJwt`, byte-equal to `shroom-cloud::mint::jwt`; golden
vector `member_vector.rs`):

```
header = {"alg":"HS256","typ":"JWT"}
claims = {"aud":"<account_id>.r2.cloudflarestorage.com","bucket":…,"exp":iat+ttl,"iat":…,"iss":"<key_id>",
          "paths":{"objectPaths":["meta/<rec>.json"],"prefixPaths":["rec/<rec>/"]},"scope":"object-read-write","sub":"<account_id>"}
jwt    = b64url(header).b64url(claims).b64url(HMAC-SHA256(UTF-8 secret, the first two parts))
cred   = {accessKeyId: key_id, secretAccessKey: hex(sha256(jwt)), sessionToken: base64std("jwt/" + jwt)}
```

- The claims are a fixed literal in serde_json's alphabetical order (C1).
- `rec` is checked against `ID` first (H6), so no credential names anything else, never
  `.shroom/`.

**Purposes** (`TTL`; R2 has no ceiling, P6):

| Purpose | TTL | Rule |
|---|---|---|
| `upload` | 604800 s | Owner (§2.4) is someone else → 403 `not-yours` (owner-class too). Deleted or sealed → 410 `deleted`. No owner: spend the budget, then `ledgerCreate`; lost to another → 403 `not-yours`. |
| `manage` | 900 s | Someone else's and the caller isn't owner-class → 403 `not-yours` (before existence). No meta or no owner → 404 `absent`. Deleted or sealed and not owner-class → 410 `deleted`. |

Then the parent. If it is missing, the Worker sets `health.json` to `blocked`/`no-parent`
(errors ignored) and answers 503 `space`/`no-parent`, after the ledger entry is written.

**Budgets** (per device per clock hour, in `rate/<did>`):

| Budget | Limit | Over it |
|---|---|---|
| `n` | 20 first upload mints (`UPLOAD_BUDGET`, H4); re-mints don't count | 429 `too-many`, `retryAfter` = seconds to the hour's end (≥ 1) |
| `k` | 60 `/key` calls (`KEY_BUDGET`, D:B125) | the same |
| `c` | 120 voice-cue judgments (`CUE_BUDGET`, D:B202) | the same |

Token Macs mint locally, with no budget.

### 3.5 Owner cookie (B25 amended)

**The cookie.**
- Value: `<b64url(JSON {m, d, t})>.<b64url(HMAC-SHA256(cookie.key, payload))>` — member,
  issuing device, unix seconds.
- Set as `shroom_owner=…; Max-Age=34560000; Path=/; HttpOnly; Secure; SameSite=Lax`.
- `.shroom/cookie.key` is base64url, ≥ 32 bytes, made once at wiring, and cached per
  isolate (a rotation lags, D:B125).

**Every use checks it live** (`cookieClaims` + `checkCookie`, one roster read, only when
the HMAC verifies): `m` and `d` are strings; `devices[d]` is active and belongs to `m`; and
`members[m]` is active.
- A cookie without `d` is a recipient.
- A bad or unreadable roster makes every cookie a recipient (C8).

**Powers.** Edit (`mayEdit`) = owner-class, or the ledger names `m`. Whoever may edit a
video may also store its Copy thumbnail picture (N46, D:B164): one GIF or JPEG per title and
kind, beside the video, under the caps in §4.17, drawn from the first frame the video shows
now (`from`, §4.17). It is the cookie's only byte write. The mark is owner-class only. A
redeem never swaps a live owner-class cookie for a member's; the grant is still spent (login
CSRF, D:B38).

### 3.6 Key confinement

Fixed keys, never taken from a record:
- `/m/<id>/<file>` reads `rec/<id>/<file>` (plus the meta and the deletion record);
- `POST /_shroom/thumb/<id>` writes only `rec/<id>/thumb-<16 hex>.gif|jpg`, the name taken
  from the bytes' SHA-256, and lists only `rec/<id>/thumb-`;
- `/d/<id>` reads only `rec/<id>/video.mp4`;
- of `.shroom/`, only `hls.min.js` is served.

The allowlist tests (`region6.test.js` "key confinement for every new route",
`worker.test.js`) drive every route with hostile input. An API route may touch only:
- `mark.json`, `cookie.key`, the roster, `health.json`, and `keys/<64 hex>`;
- its own ids' `recs/<id>`, `meta/<id>.json` and `gone/<id>`;
- the caller's `rate/<did>`;
- lists of `meta/` and `recs/`.

Page routes add `rec/<id>/…`. The join page reads the roster and `cookie.key` only.

### 3.7 What each leaked artifact can and can't do

| Leaked | Can | Ends when |
|---|---|---|
| a member's Mac | as that member: ≤ 20 new recordings/h; list, rename and delete its own; key its own pages (≤ 60/h); report `refused` on its own, `ok` on any. Never others' ids or labels, `.shroom/`, or invites | Remove (next call 403); issued upload credentials ≤ 7 days |
| a token-less owner Mac | list and delete anything; manage members; remove token-less owner Macs. Not token Macs, deploys or repair | any owner-class Remove; a fresh token always recovers |
| a token Mac | everything | roll the token, **then** a live-token Mac re-places `R2_PARENT` |
| a pending member link | one join under the invite's label | Cancel; the real invitee then sees `join-used` |
| a pending own-Mac link | owner-class for an hour (the riskiest; no Cancel row in v1) | its hour or use; then Remove… |
| a cookie | rename what it may edit, and store a picture beside it (≤ 40 per video, capped in size, D:B164); the mark if owner-class. Never list, delete, mint | removing its device or member |
| `k` | one redeem | the redeem, or 600 s |
| a recording credential | its one recording's bytes and meta. Never un-delete a sealed one | 7 days / 15 min |
| `R2_PARENT` | the token's R2 reach (no response ever carries it) | roll, then re-place |
| a false health report | flip owner pages' waiting line | the next true report or enroll |

### 3.8 Cross-site and abuse limits

- **Origin:** any `Origin` on `/_shroom/api/*` → 403 `origin`, before any read. The page
  POSTs instead need `Origin` equal to the page's own origin (else a plain 403).
- **Bodies:** a JSON object ≤ 16 KiB (128 KiB for `library`), length-capped while reading
  → 413 `too-large`; non-JSON or not an object → 400.
- **Failure paths are read-only.** The only rate counters are the per-device budgets
  (D:B125).
- **Escaping (H10):** `escapeHtml` for the join page's `inviter`/`label` (text and
  `<title>`), `escapeText`/`escapeHtml` on watch pages, `jsonForScript` for script JSON.

---

## 4. Routes

**API answers** are JSON with `noindex` and `no-store`.
- Success carries `"shroom":1`; 204 has no body.
- Errors are `{"shroom":1,"error":"<class>"}`, plus `reason`, `why`, `member` or
  `retryAfter` where noted.
- An unclassed exception (a binding error) → 503 `space`/`storage`, with no message.
- Wrong method → 405 `method`; unknown path → 404 `no-route`.
- Device-key routes read the roster first, so a bad roster answers 503 before any 401.

**The shape rule (U2; `member.rs:Space::classify`).** An answer is the space's own only if
its body is JSON with `"shroom":1` (a body-less 204 counts). Anything else is `NotShroom`,
whatever the status: workers.dev's `error code: 1042`, another Worker, an error page, an
old page's empty 405, or an undeserializable 2xx. The app then re-pings (§4.2):
- not shroom or NXDOMAIN → `join-space-missing` / `space-gone`;
- `iface < IFACE_MIN` → needs-update;
- unanswered → wait.

A status alone never maps to a state. Within `IFACE = 2`, routes and fields only grow.

### 4.1 Summary table

| § | Method | Path | Who |
|---|---|---|---|
| 4.2 | GET, HEAD | `/_shroom/ping` | none |
| 4.3 | GET, HEAD | `/join/<code>` | none (+ cookie) |
| 4.4 | POST | `/_shroom/api/peek` | code |
| 4.5 | POST | `/_shroom/api/join` | code + verifier |
| 4.6 | POST | `/_shroom/api/enroll` | parent proof |
| 4.7 | GET | `/_shroom/api/me` | device key |
| 4.8 | POST | `/_shroom/api/mint` | device key |
| 4.9 | POST | `/_shroom/api/key` | device key |
| 4.10 | POST | `/_shroom/api/library` | device key |
| 4.11 | GET | `/_shroom/api/members` | owner-class |
| 4.12 | POST | `/_shroom/api/invites` | owner-class |
| 4.13 | DELETE | `/_shroom/api/invites/<iid>` | owner-class |
| 4.14 | DELETE | `/_shroom/api/members/<mid>` | owner-class |
| 4.15 | DELETE | `/_shroom/api/devices/<did \| self>` | owner-class / any device |
| 4.16 | POST | `/_shroom/api/mark` | owner-class |
| 4.17 | GET, HEAD, POST | `/<id>` (`?k=`, `?embed=1`), `/m/`, `/d/`, `/_shroom/status/`, `/_shroom/hls.js`, `/oembed`, `/_shroom/title/<id>`, `/_shroom/edits/<id>`, `/_shroom/mark` | none / `k` / cookie |
| 4.18 | POST | `/_shroom/api/health` | device key |
| 4.19 | GET, HEAD | `/favicon.svg`, `.png`, `.ico` | none |
| 4.20 | POST | `/_shroom/api/gone` | device key |
| 4.21 | POST | `/_shroom/api/ai` | device key / parent proof |
| 4.21 | GET, HEAD | `/_shroom/cc/<id>` | none |
| 4.21 | GET | `/_shroom/named/<id>` | cookie |
| 4.22 | POST | `/_shroom/view/<id>` | none (same-site) |
| 4.22 | POST | `/_shroom/api/cleanup` | device key (`everyone`: owner-class) |
| 4.22 | GET, POST | `/_shroom/api/plays` | device key (POST: owner-class) |
| 4.23 | POST | `/_shroom/api/cue` | device key |
| 4.24 | GET, POST | `/_shroom/api/autoclean` | device key (POST: owner-class) |
| 4.24 | POST | `/_shroom/api/editing` | device key (the video's maker or owner-class) |

**The Worker writes only:**
- the roster (CAS), the ledger (create-only) and `rate/<did>`;
- `keys/<hash>` (create, CAS spend, delete);
- `health.json`, `mark.json`, `plays.json` and `gone/<id>` (plain puts);
- `views/<id>.json` (CAS), `views/salt.json` (create-only, then CAS) and `views/since.json`
  (create-only), from play beacons (§4.22, N68); `views/<id>.json` is deleted with a seal;
- `meta/<id>.json` via a title POST or a thumbnail POST (its `thumb` pointer);
- `rec/<id>/thumb-<16 hex>.gif|jpg` via a thumbnail POST: created, named by its contents,
  never anything else under `rec/` but naming's transcript below (D:B164);
- with naming on (§4.21): `rec/<id>/ai/transcript.json`, `.shroom/ai/later/<id>`, and the
  record's `title` (while untitled), `named`, `captions` and `naming`, under `If-Match`;
- `autoclean.json` (a plain put), `autoclean/state.json` (CAS) and `editing/<id>.json` (a
  plain put), and with auto-cleanup on (§4.24) the delete of a video it lists: its record's
  tombstone under `If-Match`, then every object under `rec/<id>/`, then `gone/<id>`, its
  `views/<id>.json` and `editing/<id>.json`.

It never claims a meta, and deletes bytes only as auto-cleanup's delete (§4.24).

### 4.2 `GET /_shroom/ping`

- **Response:** `200 {"shroom":true,"iface":2,"version":"0.1.9"}` (`no-store`), and
  `"ai":true` after them when the page carries Cloudflare's AI (naming on, §4.21). **No
  storage read** (the stop probe's 600 ms, DV #8). It is the one route keeping
  `"shroom":true`.
- **The app** (`Space::ping`):
  - 200 with `shroom == true` → `Shroom` (`iface` defaults to 1);
  - anything else → `NotShroom`;
  - transport → `Unanswered`, except an NXDOMAIN-shaped error, which means gone.

### 4.3 `GET|HEAD /join/<code>`

**Request.** Opening never consumes (SM:1427). The route reads the UA, the roster (once)
and `cookie.key` (only with a cookie), never `mark.json`. Other methods → 405.

**Response.** Always 200 `text/html`, `noindex`, `no-store`, `no-referrer`, CSP
`frame-ancestors 'none'` (HEAD sends headers only). `<title>` is the hero, escaped. Every
face carries the icon links (§4.19), shroom's name top-left, and "made with shroom"
bottom-right **whatever the switch says** (U6; §9 Q17).

| Face | When | Words (`COPY` / helpers) |
|---|---|---|
| unknown | not B14, or no invite has the hash | `inviteUnknown` |
| transient | roster missing, broken, unreadable | `inviteTransient` (never terminal) |
| `join-landing` | pending member invite | `joinHero`, `joinIntro`, the install card (`installLine`, Copy, `readFirst`, `macos`), [ Open shroom ] = `shroom://join/<host>/<code>`, `openFallback`, `onePerson` |
| own-Mac variant | pending `own_mac` | `ownHero`, `ownSame(host)`, the card, `ownHour` |
| owner view (best-effort, DV #31) | + a live owner-class cookie | + `ownerNote(label)`; own-Mac or label-less: `ownOwnerNote` |
| `-elsewhere` (best-effort) | UA matches `iPhone\|iPad\|iPod\|Android\|Windows\|Linux\|CrOS`; a script flips Macintosh + `maxTouchPoints > 1` | hero, `elsewhere` + [ Copy link ], the one-person/one-hour line |
| `join-expired · page` | expired or cancelled | `inviteExpired(inviter)` / `ownExpired` |
| `join-used · page` | used | `inviteUsed(inviter)` / `ownUsed` |
| "You're in" (best-effort, DV #31) | used, and the cookie's `m == used.member` (own-Mac: `d == used.device`, D:B38) | `youreIn` |

The label reaches only an owner-class cookie.

### 4.4 `POST /_shroom/api/peek`

- **Request:** `{"code":"<16>"}`. One roster read; never writes.
- **Response:**
  `200 {"shroom":1,"state":"pending"|"expired"|"cancelled"|"used","kind":"member"|"own_mac","inviter","expiresAt","now","iface":2}`.
  Never the label.

| Error | When |
|---|---|
| 400 | `code` missing or not a string |
| 404 `unknown` | wrong shape (checked after the roster read), or no invite has the hash |
| 403 `origin` | §3.8 |
| 503 `space` | the roster can't be read |

**App** (`runtime/member.rs:Lane::peek`). It pings first. Then:
- not shroom or NXDOMAIN → `SpaceMissing`;
- `pending` → `Pending{current: iface ≥ IFACE_MIN}`;
- `used` → `Used`;
- `expired`/`cancelled`/404/410 → `Expired` (inviter `None` for unknown).

It fills the `invite` file (`note_invite`).

### 4.5 `POST /_shroom/api/join`

**Request:**
`{"code","device":{"id":"d_<16>","verifier":"<64 hex>","name"|null},"previous":null|{"id":"d_<16>","secret":"<≤128>"}}`.

Validation: a bad `device`, or a non-string `code` → 400. A non-B14 `code` → 404 `unknown`.
A malformed `previous` is ignored.

**One `mutate`:**
1. **The invite by `codeHash`:**
   - none, or an unknown state → 404 `unknown`;
   - cancelled → 410 `cancelled`; expired → 410 `expired`;
   - used by the same `device.id` + `verifier` → **the same 200**, no write;
   - otherwise used → 409 `used`.
2. **`previous`** counts only if `devices[id]` exists with a matching hash.
   - That device is active → 409 `already-here`, nothing consumed.
   - **Revive** (H2) if its member isn't `m1`, has role `member` and state
     `removed`/`left`, and the invite is `member`. The member becomes active,
     `endedAt:null`, `joinedAt:now`, **keeps its old label**, and gains a `reinvites` entry
     `{invite, label, at, joinedBefore}`.
   - Otherwise ignored: never `m1`, never an active member, never across members.
3. `devices[device.id]` exists → 409 `conflict`.
4. **Pick the member.**
   - `own_mac`: ≥ 20 active `m1` devices → 409 `full`; else `m1`.
   - `member`: ≥ 200 active → 409 `full`; else the revived id, or a new `m_<16>`
     `{role:"member", label, inviter, state:"active", joinedAt, endedAt:null, invite, reinvites:[]}`.
5. **Add the device:**
   `{member, name (own_mac only), verifier, state:"active", joinedAt, endedAt:null, via: iid, tokenKey:null, proofAt:null}`.
6. **Use the invite:** `state:"used"`, `used:{at, member, device, verifier}`.

**Response:**
`200 {"shroom":1,"kind","role":"member"|"owner","member","device","inviter","space":{"host","account_id"|null,"bucket"|null,"label","subdomain"|null,"owner_name":<inviter>,"iface":2}}`.
- `account_id` and `bucket` come from `R2_PARENT`.
- `label` is the host's first label.
- `subdomain` is set only for a four-part `*.workers.dev` host.
- The member never sees its own label.

**App** (`member.rs:JoinNext::of`):

| Answer | Next |
|---|---|
| 200 | `Joined` |
| 404 `unknown`, 410 | `join-expired` |
| 409 `used` | `join-used` |
| 409 `already-here` | `join-already-here` by role (U5) |
| 409 `conflict` | a new `did` |
| `busy`/`space`/429/5xx/transport | `Waiting`: replay the identical body |
| not shroom | re-ping → `join-space-missing` or `Behind` (needs-update, D:B41 n) |
| 400, `origin`, `full`, unknown | `Bug`: logged, `join.json` kept (D:B41) |

`join.json` clears on 200, a shroom-shaped terminal answer, or `join-space-missing`; the
boot replay uses the same table.

### 4.6 `POST /_shroom/api/enroll`

**Request:** a parent proof over
`{"device":{"id","verifier","name"|null,"machine"?},"ownerName"|null}`. `machine` is
this Mac's stamp in this space (`member::machine_stamp`: sha-256 over the hardware's
`IOPlatformUUID`, the account's home folder and the host — the hardware's id never leaves the
Mac; B144), optional for older apps.

**Checked in order:**
1. body: 400 / 413;
2. header: 401 `unauthorized`;
3. `no-parent`;
4. `clock`;
5. `stale-parent`;
6. device fields (a `machine` that isn't 64 hex included), or `ownerName` > 40: 400.

**One `mutate` under `m1`** (§3.2's throttle):
- Create `m1` (`{role:"owner", name: ownerName, state:"active", joinedAt}`) or refresh its
  name.
- A new device gets
  `{member:"m1", name, verifier, joinedAt, via:"enroll", state:"active", endedAt:null, tokenKey: key_id, proofAt: now}`,
  with no cap, and `machine` when sent. **The same Mac set up again** (its folder went —
  Disconnect, an uninstall, by hand — so a new key; B144): every other active `m1` device
  that came in by enroll and carries the same `machine` is set `left` (`endedAt: now`). One
  from before stamps (no `machine`) is matched by its exact `name`, and only when its
  `proofAt` is over a day old. A token Mac set `left` this way comes back at its next enroll
  (the revive below), so a same-named other Mac is never lost.
- A known device gains `machine` when it sends one it didn't carry (a re-stamp).
- Another member's device, or another verifier → 409 `conflict`.
- An inactive device is revived, even one another owner Mac removed (U8).

Then `setHealth(ok)`.

**Response:** `200 {"shroom":1,"member":"m1","device","iface":2,"token":true}`.

**App** (`EnrollNext`):
- `clock` → correct the offset once from `Date`;
- `stale-parent` / `no-parent` → re-place (§3.4);
- `unwired` → seed once (with `ownerName`);
- `broken` → `Broken`;
- `conflict` → drop `device.key` and retry once with a fresh one
  (`runtime/cloud.rs:enroll_this_mac`);
- `busy` / `too-many` / 429 → four jittered retries;
- `storage` and others → repair;
- not shroom → re-ping.

### 4.7 `GET /_shroom/api/me`

```jsonc
200 { "shroom":1, "you": { "member", "device", "role":"member"|"owner", "name"|null },
      "space": { "host", "account_id"|null, "bucket"|null, "owner_name"|null, "iface":2 },
      "owner": { "mark":bool, "tokenHolders":n, "token":bool } }   // owner-class only
```

- `owner_name`: `m1.name` for owner-class; the member's `inviter` otherwise.
- `tokenHolders` counts the devices passing `tokenHolder`; `token` is this device's flag.
- `mark` is `mark.json`'s `on` (absent or unreadable → `true`).
- `only_token_holder` = `token && tokenHolders ≤ 1`. The count lags up to 30 days one way,
  so the app also shows the line when it can't be had (U9).

**App** (`runtime/member.rs:probe`). It pings first; `iface < IFACE_MIN` skips `/me` and
shows needs-update.
- 401 / 403 `removed` → `member-revoked`, or `owner-mac-removed` on an owner-class Mac
  (DV #21). Not debounced.
- Other refusals → `SpaceSide`, debounced.
- Not shroom → re-ping. Transport → keep the cached identity.
- A token Mac's 401 means "not enrolled": it enrolls.

### 4.8 `POST /_shroom/api/mint`

**Request:** device key, with `{"rec":"<16>","purpose":"upload"|"manage"}` (else 400, before
any key is built).

**Reads:** the roster, then the ledger, the meta and the deletion record (`metaFact`). A
first upload adds the rate object and the ledger create. The rules are §3.4's.

**Response:**
`200 {"shroom":1,"accessKeyId","secretAccessKey","sessionToken","expiresAt","bucket","endpoint":"https://<account>.r2.cloudflarestorage.com","owner":"<ledger member | m1>"}`.
The Mac copies `owner` into `meta.owner` at the claim, for readability only.

**App** (`MintNext`):

| Answer | Next |
|---|---|
| 401 / 403 `removed` | `Revoked` (not debounced) |
| 403 `not-yours` | logged; never re-minted; the take is kept |
| 410 `deleted` | the take stops: "Removed from the space" |
| 404 `absent` | (`manage`) |
| 503 `space` | `SpaceSide` |
| `busy`, 429, transport | `Backoff` |
| not shroom | `Reping` |
| others | `Bug`, never retried |

A fresh device-door credential refused 403/404 on its first PUT is space-side too; an older
one is dropped and re-minted (`runtime/uploader.rs`).

**Debounce (C7; D:B39 f).** `space-needs-owner` and `space-gone` show only after two
answers of their kind ≥ 30 s apart, with no fine answer between. A confirmed refusal sends
`POST /health {refused}`.

### 4.9 `POST /_shroom/api/key`

**Request:** device key, with `{"kHash":"<exactly 64 lower-case hex>"}`; anything else
→ 400 (H5).

**Steps:**
1. Spend the `k` budget (§3.4) → 429 `too-many`. A retry spends it too.
2. Create `.shroom/keys/<kHash>` = `{member, device, exp: now+600 s}` →
   `200 {"shroom":1,"exp"}`.
3. The key already exists:
   - an unspent grant for the same member and device → the same 200;
   - anything else (another caller's, spent, unreadable) → 409 `conflict` (a fresh `k`).

**App** (`runtime/cloud.rs:owner_key`):
- 200 → open `/<id>?k=`;
- not shroom → a token Mac's S3 fallback (§3.3);
- a token Mac's 401 → enroll and ask once more;
- otherwise → open the clean URL.

### 4.10 `POST /_shroom/api/library`

**Request:**
`{"scope":"mine"|"everyone"|"owners","known":{"<id>":"<etag>"},"knownThrough":null|"<id>","after":null|"<id>"}`.
- A bad scope → 400. A member asking for anything but `mine` → 403 `forbidden`.
- `after` and `knownThrough` must be `null` or B14, else 400.
- Bad `known` entries are dropped. The client sends ≤ 1,000 cached ids past `after`.

**`owners`.** One ledger page after `recs/<after>`, with `include:["customMetadata"]` →
`{"owners":{"<id>":"<member>"},"next"}`. `next` = the ledger cut, else the last id if the
list was truncated, else `null`. It ignores `known` and is uncapped (H3).

**`mine` / `everyone`.** The ledger and `meta/` are walked in lockstep by id:
1. List both (1,000 each). An empty truncated page is followed up to 5 times, then 503
   `busy`; it never invents an end (D:B38).
2. The end = the smallest of each truncated list's last id, `knownThrough`, and the
   ledger cut. Never the count (P3).
3. Ledger entries without `customMetadata.m` cost a GET each, ≤ 40; past 40 the page is cut.
4. Items = metas in `(after, end]`, owner = the ledger member else `m1`; `mine` keeps the
   caller's.
5. Bodies only where the ETag differs from `known`, ≤ 40 (`LIBRARY_BODIES`); the page ends
   at the 40th. Each body read HEADs `gone/<id>` beside it (§4.20).

**Response:** `200 {"shroom":1,"items":[{"id","etag","owner","meta"?}],"next"}`.
- `meta` is present only when changed (`null` if unreadable). A tombstone has no `title`;
  the app reads it as a deleted row (`shroom_upload::meta::listed`), never as unreadable.
- Tombstones come back as their meta.
- A sealed id (§4.20) comes back as a tombstone, whatever its meta says now:
  `{"schema":1,"id","state":"deleted","createdAt"}` (`createdAt` from the meta, else `null`).
- "Gone" = `known` minus every id seen, once `next` is `null`.

**Errors:** 401/403 → the cached list, only Copy link and Save to Movies live (D:B41 a);
403 `forbidden`; 413; 503.

**Callers** (`runtime/library.rs`):
- members use `mine`;
- owner-class Macs without the token use `everyone`, labels from `/members`;
- token Macs list `meta/` over S3, with `gone/` beside it (a sealed id is deleted, §4.20),
  learning owners through `owners` or a ledger GET per
  unknown id, kept in `library.json`. An unknown owner is unlabelled in *Everyone*, never
  in *Mine* (D:B54 f).

### 4.11 `GET /_shroom/api/members` (owner-class)

`200 {"shroom":1,"now","owner":{"id":"m1","name","devices":[{"id","name","joinedAt","token","self"}]},"members":[{"id","label","joinedAt"}],"invites":[{"id","kind","label","createdAt","expiresAt","expired"}],"labels":{"m_…":"…"}}`

- `owner.devices`: active `m1` devices; `token` = `tokenHolder`.
- `members`: active role-`member` members.
- `labels`: every role-`member` member ever (a revived one keeps its old label).
- `invites`: every pending invite, expired or not, both kinds, never the code. v1's face
  skips `own_mac` (U4).

Errors: 403 `forbidden`, 401 / 403 `removed`, 503.

### 4.12 `POST /_shroom/api/invites` (owner-class)

Owner-class is checked before the body and again in the CAS. `id` must be `INVITE_ID` and
`codeHash` 64 hex, else 400.

**Create:** `{"id","codeHash","kind","label","inviter"}`. `label` is required for
`member` (1–64) and ignored → `null` for `own_mac`. `inviter` is 1–40. Bad values → 400.

| Case | Answer |
|---|---|
| same id and hash | **200** with the existing row |
| same id, other hash; or the hash on another invite | 409 `conflict` |
| ≥ 20 unexpired pending, or 500 invites after eviction | 409 `full` |
| otherwise | **201**; `by: <did>`; `expiresAt` = now + 7 d / 1 h |

**Renew** (`{"id","codeHash","renew":true}`):

| Case | Answer |
|---|---|
| no id | 404 `unknown` |
| used | 409 `used` |
| cancelled | 410 `cancelled` |
| same hash | 200, no write |
| the hash on another invite | 409 `conflict` |
| expired, with ≥ 20 unexpired pending | 409 `full` |
| otherwise | 200 with a new hash, `createdAt` and `expiresAt` on the same id (any pending invite) |

**Response:** `{"shroom":1,"invite":{"id","kind","label","createdAt","expiresAt","expired"}}`.
The app builds `https://<host>/join/<code>` (`member::link_of`) and resends an identical
body after a lost reply.

### 4.13 `DELETE /_shroom/api/invites/<iid>` (owner-class)

| Case | Answer |
|---|---|
| malformed or absent | 404 `unknown` |
| used | 409 `used` with `member` |
| already cancelled | 204, no write |
| pending (expired or not) | `state:"cancelled"`, `endedAt` → 204 |

Peek and the page then read `cancelled` (the record is kept 30 days, §2.3). Own-Mac
invites are accepted, but v1 offers no control for them (§6.4).

### 4.14 `DELETE /_shroom/api/members/<mid>` (owner-class)

| Case | Answer |
|---|---|
| `m1` | 409 `owner` |
| malformed, absent, or not role `member` | 404 `unknown` |
| already removed or left | 204, no write |
| active | the member and **every active device of theirs** → `removed` with `endedAt` → 204 |

The next mint, `k` or cookie use fails. Minted credentials finish within 7 days (N32,
SM:1365). The label is kept.

### 4.15 `DELETE /_shroom/api/devices/<did | self>`

**`self`** (any device; key-only auth inside the CAS):
- unknown or wrong → 401;
- not active → 204, no write (D:B38);
- otherwise → `left` → 204. A member with no other active device leaves too; `m1` never
  changes.

`Space::leave` folds 401/403 into done.

**`<did>`** (owner-class):

| Case | Answer |
|---|---|
| the caller itself | 409 `self` |
| malformed or absent | 404 `unknown` |
| not an `m1` device | 409 `not-owner-mac` |
| not active | 204 |
| carries the token flag | 409 `token-holder` |
| otherwise | `removed` → 204 |

A removed owner-class Mac shows `owner-mac-removed` (DV #21); a fresh token revives it
(§4.6).

### 4.16 `POST /_shroom/api/mark` (owner-class)

`{"on":bool}` (else 400) → `.shroom/mark.json` = `{on, at}` (unconditional) →
`200 {"shroom":1,"on"}`. Settings · Space uses it on any owner-class Mac.

### 4.17 The page routes

HTML answers are 200 `text/html`, `noindex`, `no-store`, `no-referrer`, CSP
`frame-ancestors 'none'` (embed: `*`). Other methods → 405; unmatched → 404.

| Route | Behaviour |
|---|---|
| `/_shroom/hls.js` | `.shroom/player/hls.min.js`, immutable for a year |
| `/_shroom/status/<id>` | Public JSON, not shroom-shaped. Absent → `waiting`. Deleted or sealed → `removed`. Live with HLS, or baked (D:B197, §4.18) → `{live, poster, mp4, teaser, thumb, dur, drained}` (`thumb`: the newest Copy thumbnail picture for the title and drawn from the first frame the video shows now — the record's `thumb` with its `from` current, see the thumbnail POST — or null; D:B164. `dur`: the length in seconds, 0 when unknown, which a page opened while the video uploaded draws on its picture; D:B165) (+ `pct`, `stalled` until drained). With an edit (N63, D:B195; §4.18): `dur` is the edited length, `keep` the ranges that play, `copy` `{src, dur, poster}` once its permanent copy is current; `mp4` says Download has something to hand over, `poster`/`teaser` whether the page shows a first frame and a teaser — never one of a part that's cut (§4.18). A baked recording with no edit (D:B197: `bakedFrom` non-empty) names its own file as `copy` the same way, so a player the bake left on the original (its files 404) moves to it; a player already on that file stays. An edit that leaves nothing to play (its applied cuts cover the whole trim, or neither the record nor the list has a length) → `unavailable`, never the recording as it was. Uploading → `{uploading, pct, stalled}`. Else, or a throwing read → `unavailable` (D:B94). `pct` ≤ 99. `stalled` = `progress.updatedAt` (else `createdAt`) older than 120 s. Live, it may add `cuts` (N65, D:B200 c): `"coming"` while the space hears its takes (an `AI` binding, N66), the take went live under 5 min ago (`publishedAt`) and its edit list carries no plan mark (`edits.voice`), unless `naming.later` is set; once planned with voice cuts, a short base-36 tag of their rows, which any press changes. Never a word that was cut. |
| `/m/<id>/<file>` | No empty/`.`/`..` segment; only `m3u8 m4s mp4 jpg gif vtt` (else 404, D:B125; `gif` for Copy thumbnail's loop, D:B164). Reads `rec/<id>/<file>` beside the record: deleted or sealed → 404, record throws → 503, absent → served. While the record's edit has a current permanent copy (N63 step 3, D:B195) only the copy's own files and the recording's own first frame (`poster.jpg`, which a token-less Mac's Library fetches until the bake), while the edit keeps that frame, are served; the original's stream, `teaser.mp4`, `video.mp4` and anything else → 404, GET, HEAD and Range alike (a stale copy or no edit serves them again). A baked recording (D:B197) serves exactly the files its `media` names; with a newer copy current over it, that copy's files and its own poster while the new edit keeps its first frame. Copy thumbnail's `thumb-<16 hex>.gif\|jpg` (D:B164) are served whatever else is, so a paste already sent keeps its picture — unless an edit took something from the first 3 s the pictures show (the first frame, the teaser's first 3 s): when the record's edit or any bake (`bakedFrom[].keep`) doesn't start with a range from 0 to at least 3 s, only the record's current picture (`thumb`, its title and `from` current) is. Range → 206. `public, max-age=3600`. Media answers are `nosniff` with `content-security-policy: sandbox; default-src 'none'`. |
| `/d/<id>` | The video as it plays, as an attachment named after the title: a live record with no edit and `media.mp4` → its own file, `rec/<id>/video.mp4` (or, baked, the `edit-<n>.mp4` its key names, §4.18); with an edit, its current permanent copy → `rec/<id>/edit-<n>.mp4` (a name `editsOf` validated, never one taken as is); an edit whose copy isn't up or that leaves nothing to play, or anything else → 404 (N63). Storage failure → 503. |
| `/oembed?url=` | This origin's `/<id>` → `{version, type:"video", width, height, html (iframe ?embed=1), title?, thumbnail_*?}` (with an edit, N63: the thumbnail is its copy's own first frame once it's up, none when the copy has none; before, the recording's first frame only while the edit keeps it), else 404 |
| `/<seg>?k=<k>` | Redeem (below) |
| `/<id>?embed=1` | The player or a line, an edit applied as on the page (§4.18). The mark follows the switch; never owner mode. |
| `/<id>` | The watch page (§4.18). Cookie checked live (§3.5). A storage failure → unavailable + `x-shroom-unavailable: storage[-<code>]` (D:B94). |
| `POST /_shroom/title/<id>` | Same-origin and a live cookie, else 403. Title collapsed, trimmed, ≤ 200; empty → 400. Sealed (§4.20) → 409, whatever the meta says; no meta → 404; unparseable or deleted → 409; not `mayEdit` → 403. If-Match sets `title` and `titled:true` → `200 {"title"}`. 5 lost CAS → 503. |
| `POST /_shroom/thumb/<id>` | Copy thumbnail's picture (N46, D:B164). Same-origin and a live cookie that may edit the video, else 403. The body is the picture: a GIF (`GIF87a`/`GIF89a`, ≤ 640 wide — the teaser's own width, D:B165 — ≤ 800 KiB) or a JPEG (`FF D8 FF`, its frame header ≤ 1200 wide, ≤ 300 KiB), its first bytes agreeing with `content-type` (else 415; too wide → 400; too big → 413). `x-shroom-title` (URI-encoded) is the title it was drawn for, empty for an untitled video; it must be the record's (`titled` and `title`) at the write, else 409. `x-shroom-from` names the first frame it was drawn from, as a name under `rec/<id>/` (absent: `poster.jpg`, which a page from before the header drew from); it must be the one Copy thumbnail draws from at the write (N63) — with no edit the record's poster (`media.poster`'s file, `poster.jpg` when it names none; a baked record only its own), with an edit's copy current the copy's own (`edit-<rev>-poster.jpg`; a copy without one, none), while an edit waits for its copy none — else 409 and nothing is stored. The video must be live and not sealed (else 409; no meta → 404). At most 40 `thumb-` files per video (else 429). Stored as `rec/<id>/thumb-<first 16 hex of SHA-256>.<gif\|jpg>` (`image/gif`/`image/jpeg`); the record's `thumb` becomes `{title, from, gif?, jpg?}` under If-Match, starting over when the title or `from` differs → `200 {"url"}`. A `thumb` without `from` was stored before it and reads as drawn from `poster.jpg`; only a `thumb` whose title and `from` are current is the video's picture (the status's, the pages'). Earlier files stay, served as `/m/` says, so a paste already sent keeps its picture. |
| `POST /_shroom/edits/<id>` | A press on the owner page's voice cuts (N65, D:B200). Same-origin and a live cookie that may edit the video, else 403. Body `{cut, applied}` (`cut` `[A-Za-z0-9_-]{1,32}`, `applied` a boolean; else 400). Under If-Match: sealed or deleted → 409; no record → 404; no such cut in `edits.cuts` → 404; a hand cut (`kind: "manual"`, the editor's) → 403. Sets that cut's `applied`, moves `edits.rev` on, keeps the rest → `200 {cut, applied, keep, cuts}` (`keep`: what plays, `[[start, end]…]`; `cuts`: the status's tag), `no-store`. 5 lost CAS → 503. |
| `GET /_shroom/edits/<id>` | The owner page's strip and what plays, for the page that sees the cuts land: an owner-class cookie, or the maker's (`mayEdit`), else 403; no record or deleted → 404 → `200 {strip, keep, cuts}` (`strip`: the section's HTML, empty with no voice cuts), `no-store`. |
| `POST /_shroom/mark` | Same-origin, an owner-class cookie, `{on:bool}` (else 403) → `200 {"on"}` |

**Redeem** (`worker.js:redeem`):
1. `k` must match `^[A-Za-z0-9_-]{32,64}$`, else no reads.
2. Read the grant, `cookie.key` and the roster in parallel.
3. A grant that is absent, malformed, `spent` or past `exp` → no cookie.
4. A CAS spend; losing it → no cookie. The delete runs after the reply.
5. `checkCookie({m, d})` against the roster (a fallback grant's `role` is ignored).
6. A member's grant never replaces a live owner-class cookie.
7. Set `{m, d, t}`.

Every outcome is `302 Location: /<seg>`, with `no-store`, `no-referrer` and `noindex`.

### 4.18 `POST /_shroom/api/health` — "Waiting for your space" (§16 #44, U1)

**The record:** `.shroom/health.json` =
`{"v":1,"uploads":"ok"|"blocked","since","why":"no-parent"|"refused"|null,"by":"d_…"}`.
Only the Worker writes it. `setHealth` reads first and puts only on a change (absent =
`ok`).

**Set by:**
- `/mint` with no parent → `blocked`/`no-parent`;
- every successful enroll → `ok`;
- this route.

**Request:** device key, with `{"rec":"<16>","uploads":"refused"|"ok"}` (else 400) → 204.
- `refused` counts only from the ledger's member (the maker) or owner-class, else 403
  `not-yours` (D:B125).
- `ok` is accepted from any device.

**Senders:**
- device-door Macs send `refused` once C7 confirms while a take waits, and `ok` on the next
  progress (D:B41 m);
- a token Mac whose token died sends `refused` through its device key (D:B51 d).

**The watch page** (`worker.js:page`). "Mine" = a live cookie that passes `mayEdit`.

| Record | Viewer | Face |
|---|---|---|
| deleted or sealed | anyone | "There's no video at this link." — a plain not found, 404 (B141) |
| absent (B14 id) | mine, with a ledger entry or health `blocked` | the owner page's pending chrome (title field from the placeholder, sent once the record exists; Copy link; empty stage). Its line is the blocked words, else "Waiting for connection — your link already works." (D:B49 b, B53 c) |
| absent | anyone else | on its way: 5 s refresh, stall clock (`sessionStorage`; stalled line after 120 s, D:B94 a) |
| uploading, or live with HLS or baked | mine | the owner page: "Uploading — N%. Your link already works."; stalled and undrained, "Waiting for connection — …", or with health `blocked` "Waiting for your space — …" (owner-class) / `waitingFor(members[m].inviter)` (a member; no inviter → the connection line); drained, no line. Its title row ends with **Edit in shroom** (N69, D:B221): a plain link to `shroom://edit/<the page's host>/<id>`, drawn hidden until the upload drains (the poll brings it in), never on another's page or the embed |
| uploading | others | on its way, or the stalled line |
| live with HLS, or baked | others | the player (+ Download if `media.mp4`). It plays `video.mp4` when `media.mp4` is set and `rec/<id>/v0/stream.m3u8` (read beside the record) names one `EXT-X-MAP`; else `master.m3u8` (D:B177). The owner page and the embed choose the same way. **An edit** (`meta.edits`, N63, D:B195) rides the record already read: `editsOf` (a port of `shroom_edit::EditList::checked` + `keep`, clamped to `durationSec`, every applied cut read wherever it sits in the list) hands the player only the ranges that play (at most the first 500, `EDIT_KEEP_MAX`: past them playback ends rather than play a cut), never the list or its transcript text (`removed`, `resumes`, `why`). The player starts at the trim's in, skips each cut (every frame while playing, and on `timeupdate`/`seeking`), stops at the out, and counts its time, seek bar, ←/→ and Home/End in edited time; the edited length is the page's, the unfurl's `og:video:duration`, the status's and Copy thumbnail's. Download waits for the copy. An edit that leaves nothing to play is never read as no edit: the page, the owner page and the embed show the unavailable face, with no Download. No picture shows a part that's cut: until the copy is up the unfurl, oEmbed and the player's `poster` show the recording's first frame only while the edit keeps it (`poster.jpg` is frame 0), and no teaser (its 15 s can hold a cut), and a loaded player waits at the first range's start, so a trimmed-off first frame never shows. Once the edit's permanent copy is current (`copy.rev == rev`) the player plays `/m/<id>/<copy.file>` as it is, with the copy's own poster and teaser (page, unfurl, Copy thumbnail) — none when it has none, never the original's in their place; a player still on the original when the copy lands moves over to it when the original answers 404 (it asks the status). **Baked** (D:B197: the edit became the video — `hls: null`, `media` naming the copy's own files, `durationSec` its length, `edits` gone): the player plays `media.mp4`'s file and the page, unfurl and Download use `media`'s keys, each taken only when it sits directly under `rec/<id>/` and is a name of its slot's kind (`video.mp4` / `poster.jpg` / `teaser.mp4` or `edit-<n>.mp4` / `-poster.jpg` / `-teaser.mp4`); a record that still has its stream reads as before (a set slot is the fixed name). The stream's playlist is still read beside the record (B177: no round of its own) but a baked record's player never waits on it. `bakedFrom` never leaves the Worker. Edited again, its `edits` count on its own clock and everything above applies. **`shroom:keep`** (agreed with the voice-cut build, #59): a CustomEvent on the page's document, `detail` the ranges to play on the recording's clock, made safe as `editsOf` makes an edit (numbers only, clamped to `durationSec`, merged, at most 500) — the player takes them in place of the edit and moves a playhead in a cut on; it does nothing while the copy plays |
| else | anyone | unavailable (30 s refresh) |

The owner page polls `/_shroom/status/<id>`; its waiting words come from the render
(`cfg.waiting`). Still open: "connection" can't tell offline from asleep, and a Worker
gone or over its cap shows Cloudflare's page.

### 4.19 `GET|HEAD /favicon.svg`, `/favicon.png`, `/favicon.ico` — the tab icon (D:B109)

- **Request:** no cookie, key, query or **storage read**; answered before `/<id>`.
- **Response:** 200. `.svg` is `image/svg+xml`; `.png` and `.ico` are the same `image/png`
  bytes. `content-length`, `public, max-age=31536000, immutable`, `noindex`; HEAD sends
  headers only.
- The art is `site/favicon.{svg,png}`, in the module, held equal by `icon.test.js`.
- Every `/<id>` and `/join/` face links all three; the embed links none. The icon ignores
  the mark switch (Bagrat's to rule).

### 4.20 `POST /_shroom/api/gone` — sealing a delete (D:B125)

**Why.** An upload credential can rewrite its meta for seven days (N32), so a tombstone
alone isn't final. `.shroom/gone/<id>` is out of every recording credential's reach.

**Request:** device key, with `{"rec":"<16>"}` (else 400).

**Steps:**
1. Read the ledger, the meta and the deletion record.
2. Already sealed → 204.
3. Neither a ledger entry nor a meta → 404 `absent`: sealing an id nobody has recorded
   would refuse its maker's first mint.
4. Someone else's (§2.4) and the caller isn't owner-class → 403 `not-yours`.
5. A meta that isn't `deleted` → 409 `not-deleted`.
6. Otherwise put `gone/<id>` = `{"v":1,"by":"<did>","at"}` → 204.

A token Mac writes the same key over S3 with `by:"token"` (`runtime/library.rs:seal_here`).

**Effect.** `readMeta`, `metaFact`, the title edit and the Library's body reads HEAD the
record. A sealed id reads as deleted on the page, embed, status, `/m/`, `/d/`, `/oembed`
and the Library (§4.10); its title edit is 409; every mint on it but an owner-class
`manage` is 410. A sealed id with no ledger counts as `m1`'s. A token Mac's own listing
(`shroom_upload::library::refresh`) lists `gone/` beside `meta/`: a sealed id is a deleted row
there too, with no read.

**Callers.** `runtime/library.rs:delete` seals after `library::delete` and a tombstone
check; a seal that fails (an older page) is logged and the delete stands.

### 4.21 Names, chapters and captions (N66, N67; D:B205–B209)

**When.** Only on a page deployed with an `ai` binding (`AI`) and a Cron Trigger each minute
— the setup Mac's switch (`shroom_cloud::wire::set_ai`). Without the binding the AI route
answers `{"shroom":1,"ai":false}` before reading anything, the schedule does nothing, and
no page carries any of it.

**`POST /_shroom/api/ai`** — `{"rec":"<16>","through":<pieces>}` (another minute is up:
`through` pieces of the take are up, unbroken from the first) or `{"rec":"<16>","done":true}`
(the take is live). The device key, or the setup Mac's parent proof (§3.2, as `enroll`). Only
the take's maker or an owner-class caller (§2.4): else 403 `not-yours`; no ledger and no
meta → 404 `absent`; a tombstone → 410 `deleted`; a bad body → 400. One step a call:
- `{"ai":true,"step":"heard","more":bool}` — a window of ten pieces heard (the last, partial,
  once done); `more`: another step can be taken now;
- `{"ai":true,"step":"titled","more":bool}` — the early title (D:B205 f): once the first minute
  is heard while the take records, a title from it alone (tried once a take);
- `{"ai":true,"step":"named","more":false}` — the done take named;
- `{"ai":true,"step":"idle","more":false}` — nothing to do yet;
- `{"ai":true,"later":"daily"|"busy"|"error"|"running"}` — it waits: the free day used up
  (until 00:00 UTC), a busy model (five minutes), another failure (a little longer each time;
  after five, the take is let go), or another step holds it.

Every answer but idle also carries `note`: one short line for the Mac's log (the window heard,
the early title, the name with its model, time, chapters and tokens, or the wait with Workers
AI's code alone; never a word said or a title, B206 f).

The app's caller is `shroom_cloud::member::ai_signal` (asked again while `more`).

**The schedule** (`scheduled`): the marks under `.shroom/ai/later/<id>` whose `after` (custom
metadata, beside `since`) has come, oldest first; one step a tick.

**Stored.** `rec/<id>/ai/transcript.json` = `{v:1, model, language, duration, heard_through,
done, windows, clock, words:[[text, start, end],…], tail}`: seconds on the recording's clock
(`clock`: the last bake it's on, null for the original), final up to `heard_through`, the
window's last 1.5 s in `tail` until the next window hears it. Whisper is asked with
`vad_filter: true` and `initial_prompt: AI_HEAR_PRIME` (the cue and the hesitations, kept in
the words for voice cuts; captions leave lone hesitations out), and a word with no letter or
digit isn't kept (B205 a, e). The mark
`.shroom/ai/later/<id>` = `{v:1, since, after, done, through, rename, reason, tries}`. The
record: `title` (only while not `titled`), `named` = `{by:"ai", model, at, rev, clock,
words, title, chapters:[[start, title],…], language, ms, tokens?:[in, out], fallback?}`
(`fallback`: the first model's code when the second answered; with `early: true` and no
chapters while it's the early title, until the full name at done replaces it; `failed: true`
and `code` for a take let go), `captions` = `{language}`, and `naming` = `{later:"daily", at,
code}` while the free day is used up. The transcript's `early: true`
says the early title was tried.

**Bakes.** Whoever bakes an edit (N63: the editor's copy becomes the video, `meta.edits`
dropped) appends to the record's `bakedFrom` in the same write: `[{at, keep:[[start, end],…]},
…]`, one entry per bake in order, `at` a string naming the bake and `keep` the ranges kept, in
seconds on the clock before it (a lone `{at, keep}` reads as a list of one; the editor keeps
the last 20). It's the only way the words, chapters and captions cross a bake: each is carried
through the bakes made since its own `clock` (a word goes when most of it went), and the
transcript is rewritten on the new clock once, after a view. A baked record without it keeps
its words on the old clock; words or chapters on a clock the list no longer holds aren't shown.

**After a view** (in `waitUntil`): an edit saved since the naming (`named.rev` ≠ the bakes
and `meta.edits.rev`) that removes at least a fifth of the words it saw, or a whole
chapter, marks the take to be named again; else the revision is stamped.

**`GET|HEAD /_shroom/cc/<id>`** — the captions as WebVTT (`text/vtt`, `max-age=300`), written
per view from the transcript through the saved edit: `e=1` on the edited clock (the editor's
own copy plays), otherwise on the recording's. 404 without captions, for a deleted record.

**`GET /_shroom/named/<id>`** — the owner page's ask while a name is on its way: the maker's
or an owner-class browser (§3.5) only, else 403: `{"named":"<title or empty>","titled":bool,
"later":bool,"early":bool,"done":bool,"chapters":[[start,"title"],…],"captions":{"src","lang"}|null}`.
`done` once the full name is in (or naming gave up); `chapters` and `captions` are what a
reload would put in the player (the bar's clock; the track on the recording's, the page adding
`&e=1` when an edit's own copy plays). The page asks on until `done`, puts the chapters and
captions into the player as it plays, and asks on till the player is there (D:B239).

A seal also deletes the video's plays, `views/<id>.json` (§4.22).

### 4.22 Plays, and the suggestions to clean up (N68, D:B210–B214)

**Counting.** A viewer's page and the embed (never the owner page) run `VIEW_JS`: at the
first play, `navigator.sendBeacon("/_shroom/view/<id>", {"e":"play","t"})`; each time the
tab hides, `{"e":"end","t","f"}`, `f` the share watched (the union of `video.played` over
the length, 0–1). `t` is a random token held only in that page's memory. No cookie, nothing
stored on the viewer's device.

`POST /_shroom/view/<id>` takes at most 512 bytes. It refuses with 403 an `Origin` that
names another site, or `Sec-Fetch-Site: cross-site`. WebKit sends `Origin: null` from a
page being left, so that is taken. A malformed body gets 400. Everything else gets 204, and
the beacon learns nothing. It writes only when the space counts plays (`plays.json`'s `on`,
true unless set false) and the video is live. A guessed id never makes a record.

**Stored:** `.shroom/views/<id>.json`, by CAS (≤ 5 tries):

```jsonc
{ "v": 1, "plays", "viewers", "ends", "watch": [10 counts by tenths watched], "watchedSec",
  "firstAt", "lastAt", "days": { "YYYY-MM-DD": plays },     // the last 400 days
  "today": { "d": "YYYY-MM-DD", "h": [viewer hashes], "t": { "<token>": tenth } } }
```

- A token's play counts once. Its latest end replaces the one before.
- A viewer is the first 8 hex of SHA-256(the day's salt, `cf-connecting-ip`, the
  user-agent, the id). The hashes and tokens are kept for that day only, ≤ 1,000 a day.
- `.shroom/views/salt.json` = `{d, salt}` is replaced when the day turns, so yesterday's
  hashes can't be recomputed.
- `.shroom/views/since.json` = `{since}` is written once, at the space's first record. A
  video made before counting began is quiet only from then on.

**`POST /_shroom/api/cleanup`** — device key, `{"scope":"mine"|"everyone","formula"?,"after"?}`
(`everyone` is owner-class only, else 403; an unknown formula is 400). It pages as §4.10
does: ≤ 40 records a call, `next` until null. The answer is
`{countingSince, counting, formula, views, items, next}`. `views` is counting's health over
the page, for the Mac's run log: `{records, unreadable, newestPlayAt}`, the plays records
read, the ones that wouldn't read (that video then reads as unplayed), and the newest play
any of them saw. The Mac logs each read in two lines (`shroom_cloud::cleanup_log`): the
counts, sizes and that health, then each candidate by a short id, its reason, score and
facts. Each live item carries:

- `id, title, createdAt, owner, ageDays, durationSec`;
- `bytes`: the record's, which the Mac writes when the upload drains; else the length ×
  10 GB / 3 h;
- `usdPerMonth, plays, viewers, medianWatched, lastPlayedAt, plays90`;
- `quietDays`: null while counting can't tell yet;
- `keep`: the record's, which Pin sets (and Unpin clears) under If-Match;
- `candidate, score, reason: {code, …facts}`;
- `deletesAt`: when auto-cleanup deletes it (§4.24), or null.

The answer also carries `autoclean: {days, last}`: the setting's days (0: off) and the newest
round auto-cleanup deleted, scoped as §4.24's `GET` scopes it.

The default formula is `fading-cost`. Its candidate is at least 14 days old (30 for the
score), not played in 14 days, quiet 30 days, never kept, with heat < 0.25 and ≥ 1¢ a month
at list price. Heat is plays halving every 30 days, × (0.3 + 0.7 × median watched). The
score is cost ÷ (heat + 0.05). `idle`, `fading`, `cost` and `gravity` stay for comparison
(`history/proposals/cleanup-suggestions.md`). The reason codes are `never-played`,
`stopped-early`, `not-lately` and `big-and-quiet` (cost only), or, when the video isn't a
candidate, `kept`, `new`, `watched-lately` and `counting-too-new`.

**`GET /_shroom/api/plays`** → `{on}`, for any device. **`POST {on}`**, owner-class only,
writes `plays.json` = `{on, at}` → `{on}`. Turning counting off keeps what was counted.

**Callers.** `runtime/library.rs`: `LibCmd::Suggest` pages `cleanup` to its end and keeps
the candidates in score order, with the bytes of every item read: the space's size on an
owner-class Mac. `LibCmd::Plays` posts the switch, and `LibCmd::Keep` sets the record's
`keep`. All Macs, the token Mac too, use the device key.

### 4.23 `POST /_shroom/api/cue` — the judgment above the voice-cue planner (D:B202)

**Why.** A line said again in other words shares too few words with its slip for the
planner to line them up (N65 (2)). The maker's Mac lists where the redone stretch might
begin; the space's own Cloudflare AI (N66's binding) weighs each; the Mac picks.

**Request:** device key, with `{"rec":"<16>","cue":<integer ≥ 0>,"after":"…","candidates":["…",…]}`:
`after` is what was said after the cue, each candidate the words from one possible start up
to the cue, nearest first; 1 to 6 candidates, each text at most 2,000 characters. Anything
else → 400, before the ledger is read or the model called.

**Steps:**
1. Someone else's take (§2.4) and the caller isn't owner-class → 403 `not-yours`.
2. No AI bound (a space that doesn't listen) → `{"later":true,"why":"no-ai"}`, spending nothing.
3. Spend the device's `c` budget (§3.4): over it → 429 `too-many`.
4. One Clef call (`@cf/cloudflare/clef`): the words as its state, never its questions; one
   `noul` question per candidate (is everything from this start said again after the
   command?).
5. → `{"scores":[…],"ms":<n>}`, one score per candidate in its order, each held to 0–1, one
   Clef left out 0; `ms`, how long Clef took, for the Mac's log. A model error →
   `{"later":true,"why":"daily"}` (the free day used up: `3036`, `4006` or its words, as the
   AI build reads them) or `{"later":true,"why":"busy"}` (anything else).

It writes nothing but the budget: the words go to the model and back, never to the bucket
or a log.

**Callers.** `runtime/voice.rs`, through `shroom_upload::voice::after_go_live_judged` and
`look_again_judged`, once per question of a take's plan. `Space::judge` takes only one score
per candidate, each 0–1, and `shroom-edit` cuts from the farthest start scored 0.4 or more
(`JUDGED`). `later`, a refusal or no answer: the cue keeps the code's own plan. The Mac's run
log keeps, per cue, the scores and `ms`, or the `why` (`later: busy`, `refused 429`, `no
answer`), never the words.

### 4.24 Auto-cleanup (N75)

Off by default. When it's on, the space's own page deletes the videos nobody has watched in
15, 30 or 60 days, or a year, on its own schedule, so no Mac has to be open (Bagrat, 2026-10-24;
a year since 2026-10-25, N77). A
pinned video (`keep`) is never deleted, and every video is listed three days before it goes.

**The setting**, `.shroom/autoclean.json` = `{v:1, days, at, since}`: `days` is 0 (off), 15,
30, 60 or 365 (a year); `since` is when it was last turned on or shortened (a change to a longer wait
keeps it). A missing file reads as off.

- **`GET /_shroom/api/autoclean`**, any device → `200 {days, since, going:[{id, deletesAt}],
  last}`. `going` is what's listed, earliest first: owner-class Macs see every video, a
  member only those the ledger says are theirs. `last` = `{at, ids, bytes}`, the newest
  round's deletes (a round is a day), or null; a member sees only their own videos in it and
  their bytes, and null when none were theirs.
- **`POST {days}`**, owner-class only (else 403); `days` not 0, 15, 30, 60 or 365 → 400. It writes
  the setting, and the state's walk starts afresh (`scanAt: 0`, `cursor: null`); off also
  empties `going`. → `200 {days, since}`.
- **`POST /_shroom/api/editing {rec}`**, the video's maker (by the ledger) or owner-class
  (else 403) → puts `.shroom/editing/<rec>.json` = `{until: now + 15 min, by: <device>}` →
  204. A Mac sends it every five minutes while an editor is open on that video. Auto-cleanup
  never deletes a video whose mark is still good.

**The schedule.** The same single `* * * * *` Cron Trigger the AI's schedule is (§4.21, D:B206
c): the Mac that holds the setup key puts it while AI *or* auto-cleanup is on, and takes it
away when both are off. Each run takes one step (`autocleanTick`), and only when the
setting is on and counting plays is on (`plays.json`): with counting off it pauses, since it
can't see plays. The step's record is `.shroom/autoclean/state.json` = `{v:1, scanAt, cursor,
going:{<id>:{warnedAt, deletesAt}}, last:{at, ids, bytes, sizes}}`, written under `If-Match`
(a lost race leaves the step to the next run).

- **The walk**, while `cursor` is set or once a day: the next `AUTOCLEAN_SCAN` (10) records
  of `meta/` after the cursor, each with its plays record. A video is due `days` after the
  latest of its last counted play (`lastAt`), its `createdAt` and when counting began
  (`views/since.json`; without it nothing is due). A live, unkept video due within three
  days is listed, `warnedAt` the first run that listed it, and
  `deletesAt` = the latest of its due date, `warnedAt` + 3 days and `since` + 3 days. Any
  other one leaves the list. At the listing's end, `cursor` is null and `scanAt` now.
- **The delete**, on the other runs: the listed videos whose `deletesAt` has passed,
  earliest first, a few at most. Each is read fresh. One that's sealed, deleted, no longer
  live, kept, not fully uploaded (its `progress` short of `total`), or played since
  `warnedAt` leaves the list. One whose editor's mark is still good waits for a later run.
  The first that's none of these is deleted, as a Delete is (`shroom_upload::library::delete`,
  then §4.20's seal): the tombstone `{schema:1, id, state:"deleted", deletedAt, createdAt,
  owner, by:"auto"}` under `If-Match`, then every object under `rec/<id>/` (listed and
  deleted a thousand at a time), then `gone/<id>` = `{v:1, by:"auto", at}`, then its plays
  record and its mark. It joins `last`. One delete a run at most.
- **Timing:** on the date, or within a day of it.

**Cloudflare's Free plan** (the Workers limits page, read 2026-10-24): 5 Cron Triggers per
account, of which this uses one; 10 ms of CPU per scheduled run, which is why the walk takes
ten records a run; 50 subrequests per run, R2 calls included (a walk's step is about 22, a
delete about 10 plus a list and a delete per thousand objects); and 15 minutes of wall time
per run.

---

## 5. The Mac's disk

`~/.shroom` is mode 700, and its files are 600, written atomically with `"v":1`. The
whitelist is `runtime/home.rs:FILES`: `prefs.json`, `once.json`, `space.json`,
`token.json`, `device.key`, `primed.json`, `screen-asked.json`, `status.json`, `restore.json`,
`library.json`, `invite`, `join.json`. No secret, code or `k` reaches a log, IPC slice, Scene or argv.

| File | Contents | Lifecycle |
|---|---|---|
| `invite` | `{v, code, host, inviter?, own_mac?, expires?}`; the installer's `{v, link, arrived}` normalized on read (`InviteFile::read`) | written on arrival, filled by peek; cleared at join's 200, Stay / ✕ / a dead face closed (D:B39 c), the wipe |
| `join.json` | `{v, host, code, device, secret, name?, previous?}` | written **before** `POST join`; replayed at boot (D:B41 g); cleared on 200, a terminal answer, `join-space-missing` |
| `device.key` | `{v, host, device, secret}` | from `join.json` at 200, or `ensure_device_key` before a token Mac's first enroll; **kept while revoked** (for `previous`); the wipe |
| `space.json` | `SpaceIdentity` (`shroom-core/src/world.rs`): `account_id, label, subdomain, host, bucket, role` (`TokenHolder`\|`OwnerClass`\|`Member`)`, member_id, owner_name, account_name?, foreign_v?`. **No device id.** | join's 200, or wiring; the wipe |
| `token.json` | `{v, token, account_id, several}` | token Macs; the parent pair is derived, never stored |
| `library.json` | the cache + `owners` (id → member) | the Library lane |
| `takes/<dir>/cred.json` | `{v, purpose:"upload", accessKeyId, secretAccessKey, sessionToken, expiresAt, bucket, endpoint}` | upload start, re-mint; removed by the wipe and a switch |
| `takes/<dir>/voice.json` | the voice lane's mark (N65, D:B201): `{state:"planned", at, cues}`, `{state:"skipped", at, why}` or `{state:"waiting", since}` | written after the go-live and by the sweep; goes with the take |
| `posters/<id>.jpg` | thumbnails (D:B54 g) | the Library lane |

`manage` credentials and `k` are memory-only. Owner names = `os::short_user_name()`.

**Act by act:**
- **Join:** `invite` → `join.json` → (200) → `device.key` → `space.json` (`Member`, or
  `OwnerClass`/`m1` for own-Mac) → delete `invite` and `join.json`.
- **Switch** (`runtime/member.rs:Lane::land`): the old files stay until the new 200. Then
  the new `device.key` and `space.json` are written (a failure restores the old key and
  keeps `join.json`). Then `token.json`, `library.json`, `restore.json`, `posters/` and the
  takes' `cred.json` go, and the old host is deregistered with the old key (best-effort,
  3 s).
- **Token Mac:** `device.key` → enroll (after `Wired`, non-fatal) → `space.json`
  (`TokenHolder`, `m1`).
- **Leave / Disconnect** (`runtime/mod.rs:wipe`): `DELETE devices/self`, not waited on.
  Then delete `space.json`, `token.json`, `device.key`, `library.json`, `invite`,
  `join.json`, `restore.json`, `posters/`, every `cred.json` and the stash. The prefs,
  `once.json`, `primed.json`, the takes and the login item stay. The core refuses while
  uploads drain or a Movies save runs (D:B54 c).

**Bucket writes by Macs:**
- wiring: `cookie.key` and the roster (create-only), the manifest last;
- the maker's Mac: a take's voice cuts into its record's `edits`, under If-Match, once (N65, D:B201);
- token Macs: ledger entries, `gone/<id>`, fallback grants;
- the sweep: deletions (§6.8).

**Bindings, complete:** `BUCKET` (`r2_bucket` → `shroom-space`) and `R2_PARENT`
(`secret_text`).

---

## 6. Flows, step by step

### 6.1 Invite (owner-class Mac)

1. Type a name; press Invite or Return. The lane makes the code, an `i_` id and
   `codeHash`, then calls `POST invites` (`inviter` = the short name).
2. 201 → the row "Ani — invited just now", the link pill and [ Copy link ]. Nothing is
   auto-copied (law 10).
3. Transport failure → the text stays; a retry resends the identical body. A refusal reads
   "Couldn't make a link just now — try again in a moment." (DV #28).
4. Later rows come from `GET members` ("invited <date> · N days left", days rounded up).
   Expired → [ New link ] → `renew`. The plaintext link is shown once.

### 6.2 Join — three doors, one pipeline (N29–N31)

**Doors:** [ Open shroom ], the installer's file + `--invite` (§6.12), and a paste anywhere in the welcome or Settings → Space (no door shows, DECISIONS B159).
`member::parse_link` takes the bare, `https://` and `shroom://join/<host>/<code>` forms
(a dotted host, a B14 code; the host is lower-cased).

**Arrival.** Write `invite` (waiting while a take is open, law 4). Then peek (§4.4):
- not shroom → `join-space-missing` at once (DV #20);
- offline → join-waiting's offline face until peek answers (DV #20);
- online but unanswered → the spinner (nameless line, D:B41 j);
- dead → `join-expired` / `join-used`;
- pending but `current:false` → the needs-update line with Join greyed (D:B41 n; DV #30);
- pending, and this Mac has:
  - no space → `member-welcome`;
  - the same host → `/me`: 200 → `join-already-here` by role (U5); 401/403 →
    `member-welcome` with `previous`;
  - another host, or half set up → `join-switch-confirm`. Never-drainable takes don't
    block it; they are named and offered Movies (D:B41 b). Draining or recovered takes do
    block it. The only-holder line shows unless `/me` says otherwise (U9).

**Join:** write `join.json` → `POST join` ("Joining — a few seconds."). 200 → §5 →
`all-set`. Other answers follow §4.5. No credentials are minted at join.

### 6.3 Re-invited after removal

The revoked Mac kept `device.key`, so its join sends `previous`. The same member id comes
back under its **old label** (the owner's row becomes "Mher — joined <re-join date>"), and
the waiting takes drain because their ledger entries name that member (D:B41 b). After 180
days the dead device is pruned: the Mac joins as a new member, and its old takes read
`not-yours`.

### 6.4 Add another Mac (N33)

1. `POST invites {kind:"own_mac", label:null}` (one hour) → the nest with the link and
   "…good for one hour."
2. The other Mac joins with the own-Mac words and lands under `m1` with its sharing name.
3. It appears as the sub-row "You · MacBook Air — joined …".

While the link is pending there is no row and no Cancel (U4; §9 Q16).

### 6.5 Remove a member, cancel an invite, remove another Mac

All from `member-remove-confirm` (no Return default, law 9):
- a member → `DELETE members/<mid>`;
- a pending member invite → `DELETE invites/<iid>` (409 `used` means they won: re-fetch);
- your other Mac → `DELETE devices/<did>`, never offered for self or a token-holder
  (SM:1394-1399).

A failure reads "Couldn't remove that just now — try again in a moment." (DV #28).

### 6.6 Leave, Disconnect, switch

§5's wipe and switch.
- The owner sees the row go; the videos keep the label (SM:1564-1565).
- Offline, the stale row stays until removed.
- A token Mac's row is unremovable until its 30-day flag lapses.

### 6.7 Member recordings and uploads

1. **Capture start** (DV #4): `POST mint {rec, upload}` → `cred.json`. A token Mac writes
   the ledger over S3, then mints locally.
2. **Upload** (`shroom-upload`, D:B49):
   - the create-only claim of `meta/<id>.json` first (`owner` = mint's `owner`);
   - poster, segments, playlists, master, the flip to `live`;
   - then the single-file copy and teaser.

   `progress` is rewritten at most every 5 s and at least every 30 s while bytes move.
   Every rewrite uses B21 If-Match and stops on `deleted`.
3. **Expiry** → re-mint the same `rec`. A credential with over an hour left finishes after
   a removal (N32, D:B52 f).

**Faces** (law 1; each clears by itself):

| Cause | Face | Start |
|---|---|---|
| 401/403 `removed`, a member's Mac | `member-revoked` | greyed |
| the same, owner-class without the token | `owner-mac-removed` (DV #21) | greyed |
| space-side (mint, `/me`, a fresh PUT), twice ≥ 30 s apart | `space-needs-owner` (owner-class: `space-degraded~no-token-mac`) | live |
| ping not shroom / NXDOMAIN, twice ≥ 30 s apart | `space-gone` | greyed |
| `iface < IFACE_MIN` | the needs-update line (SM:1266-1270) | live |

### 6.8 Member Library and delete scope

- **Members:** `library {mine}` pages to `next:null`. Rename and Delete… mint `manage` per
  id (memory only), run `shroom-upload::library::{rename, delete}`, then seal (§4.20).
- **Owner-class without the token:** `everyone`, with `manage` on any id.
- **Token Macs:** an S3 list plus the ledger map (§4.10).

**The cleanup sweep** (`shroom-upload::library::cleanup`, D:B91).

*When:* token Macs only, over the token's pair, on its own lane — 10 min after a sound
probe pass, then every 6 h awake. Never offline, mid-take, or while the space reads
broken.

*It reaps:*
- bytes under a tombstone (read from the meta);
- record-less bytes when `reapable`: a ledger entry ≥ 1 day old, the id in none of this
  Mac's take folders, and either made here and no longer held, or its maker device out of
  the roster > 7 + 1 days (an unreadable roster reaps none);
- page keys spent, more than an hour past `exp`, or not grants.

*It never touches* records, tombstones, deletion records, the ledger, the roster or rate
objects.

### 6.9 Member owner-mode cookie, and `k` off argv

1. Stop, Get link, Open page or the deferred open: make `k`, `POST key {kHash}`, then open
   `/<id>?k=<k>` through **NSWorkspace `openURL:`** (`runtime/cloud.rs:open_privately`),
   never `open` argv. `k` is never logged.
2. The redeem sets `{m,d,t}` and redirects to the clean URL.

A member's own videos get the title field and no mark switch (N40). Removing the device or
member ends the cookie on its next use.

### 6.10 Page version (`iface`)

- `worker.js`: `IFACE = 2`, `VERSION = "0.1.9"` (both in ping). `member::IFACE_MIN = 2`.
  Within an iface, routes and fields only grow; a break takes a new path and an iface bump.
- **`VERSION` moves with every page change** (D:B61). `page-version.test.js` holds
  `worker/page-version.json`'s fingerprint of `src/worker.js` + `vendor/hls.min.js` per
  VERSION. It equals the app's version (`tauri.conf.json`;
  `the_bundled_pages_version_is_the_apps`).
- **Deploys compare `VERSION`, never `iface`.** `wire::may_deploy_over` allows only strictly
  older pages, never an unreadable one (D:B50). Wiring keeps a current page and re-places
  the parent alone. `wire::refresh_page` (token Mac, once a launch) seeds, PUTs and
  enrolls only over an older page; an unanswered ping is retried, never "checked"
  (D:B53 a).
- **Other Macs only read:** `iface < IFACE_MIN` → the needs-update lines (SM:1266-1270),
  by role, or by the invite's kind at join (DV #30).

### 6.11 Wiring and update additions (§16 #46/#47)

**Wiring** (`wire::wire_with`), in order:
1. `worker_fact` (a foreign Worker → `WireError::Foreign`);
2. the bucket, then the player assets;
3. `cookie.key` and the roster (`registry::seed`), both create-only;
4. the script PUT with `BUCKET` + `R2_PARENT` — or, over a current page, `R2_PARENT` alone;
5. the workers.dev route;
6. ≤ 90 s (every 3 s) for the page to answer as shroom;
7. the self-check;
8. **the manifest, last** (create-only).

Enroll follows `Wired`, non-fatal.

**Token-Mac entry.** After a sweep pass proves the token live, the Mac enrolls
(`runtime/cloud.rs:enroll_on_entry`); most entries write nothing (C5). It also enrolls on a
401 to its device key (D:B42). A dead token never re-places.

**Spaces wired before Region 6** get the roster from the page refresh, before an iface-2
script; until then the routes answer 503 `unwired`.

**A `broken` roster** fails everything closed:
- device-key routes → 503, cookies → recipients, the join page → its transient face;
- joins wait;
- enroll → `EnrollFail::Broken`.

`registry::set_aside` has **no app caller yet**. A reseed waits on §9 Q19.

### 6.12 The installer's hand-off (H9; amends B29)

`installer/install.sh` step 4: if `--join` carried a link-shaped value
(`[A-Za-z0-9._:/-]`), it writes `{"v":1,"link":"…","arrived":"iso"}` under `umask 077`
(tmp, then `mv` to `~/.shroom/invite`) and runs
`open "$APP" --args --relaunched=<how> --invite`. Any other value writes nothing and opens
plainly.

- The link never rides the app's argv.
- Without `--invite`, an `invite` file is remembered, not new (D:B39, B45).
- Mid-take, the script refuses before writing.

---

## 7. Tests

### 7.1 L1 — Worker unit tests (`worker/test/`, vitest, `fake-bucket.js`)

`fake-bucket.js` answers as R2 does: `onlyIf` as `R2Conditional` or `Headers` (`null` on
failure), create-only, `customMetadata` (listed only with `include`), Range. Tests inject
short truncated pages (`pageSize`), `preconditionThrows`, `refuse(key, n)` (C5),
`hooks.afterGet`/`beforePut` (racing writers), and read its `ops` log.

- `region6.test.js` is this interface's suite: every §4 API route, `mutate`, pruning and
  caps, the golden vector, redeem and the cookie, deletion records, budgets, health, the
  join page, escaping, key confinement, "secrets never leave".
- `worker.test.js`: the read routes' allowlist, `/d/`'s fixed key, `nosniff`/sandbox.
- `owner.test.js` (the mark, D:B108/B110), `page.test.js` and `faces.test.js` (D:B49,
  B94), `icon.test.js`, `copy.test.js`, `unfurl.test.js`, `latency.test.js`,
  `vendor.test.js`, `page-version.test.js` (§6.10).
- `edits.test.js` (N63, D:B195): `editsOf` and the player's own arithmetic held to
  `crates/shroom-edit/tests/keep-vectors.json`; bad data (not numbers, not objects, a
  100,000-cut list); the page, embed, owner page, unfurl, oEmbed, status, `/d/` and `/m/`
  with no edit, an edit, a stale copy and a current one; step 3's 404s; no read of its own;
  no transcript text in anything a viewer gets; the player's skip, time, seek bar and keys,
  and its move to the copy, against a stand-in video; a baked recording (D:B197) and its keys,
  edited again; `shroom:keep` and its sanitizing; Copy thumbnail's picture following the edit
  (`from`, what `/m/` serves).
- `thumb.test.js` (N46, D:B164): the thumbnail POST and its checks, `from` among them, and
  the page's own drawing against a stand-in browser.
- `workerd/contract.workerd.mjs`: the same file under workerd (an edited record and its copy
  among it).
- `dev/page-drive.mjs` (`npm run drive:page`, not CI): the page in headless Chromium and
  WebKit, an edited record among it, played at 2× and sampled so a cut moment never shows;
  a baked recording; ranges handed over by `shroom:keep`.

Rust: `shroom-cloud` units (shape rule, `*Next` tables, `parse_link`, proof, redaction),
`member_vector.rs` (golden vector), `registry_fake.rs`, `wizard_fake.rs`,
`wire_faults.rs`, `shroom-core/tests/region6.rs`, and the app's drives
(`app/src-tauri/src/drives.rs`).

### 7.2 L2 — live, fenced probes (retired 2026-10-14)

They ran on the main account, fenced to `shroom-test-…` names, until main-account testing
retired (TESTING-PLAN D3); the files are in git before that date. `registry_probes_live.rs`
passed P1–P8 on 2026-10-02 (D:B35):
- P1: one conditional-put winner; 3.4 writes/s to one key were not refused.
- P2: create-only works in both forms. P3: `include` returns metadata.
- P4: a minted credential can list and delete inside `rec/<id>/`, 403 outside.
- P5: `secret_text` can be re-placed alone.
- P6: 7- and 30-day credentials are accepted (shroom caps at 7).
- P7: an absent Worker answers 404 `error code: 1042`.
- P8: 600 binding operations fit in one invocation.

`region6_live.rs` was P9, end to end. Today the full e2e on the test account
(`e2e-full.yml`) wires a space with the app's own wizard and takes a member through invite,
join, record and removal; the fake's contract with Cloudflare is
`tests/e2e-live/tests/contract.rs`; the rarer Region 6 paths (re-invite with `previous`,
the stale parent, leave) are held on the fakes.

### 7.3 Build order

Done. See history/design/WORKER-INTERFACE.md §7.3.

---

## 8. Rejected alternatives

See history/design/WORKER-INTERFACE.md §8 and each DECISIONS row's rejected column.

---

## 9. Open questions for Bagrat

All still his. "Built" = built provisionally, as recommended.

1. The registry store is R2, reopening R10's "never in the bucket". Built (D:B37; P1, D:B35).
2. The Worker's boundary: read, mint, its own records; no byte deletes, no meta claims. Built (D:B37; `gone`, D:B125).
3. Ownership by the ledger, accepting the offline-capture residual (§2.4). Built (D:B37, B42).
4. The B25 amendment: `{m,d,t}` checked live, grants via `/key`, the token fallback, a broken roster failing closed. Built (D:B37, B38).
5. The best-effort join-page cookie faces and `-elsewhere`. Built (DV #31).
6. Arrival-time dead faces and the offline arrival. Built (D:B39 a; DV #20).
7. Pending links can't be re-copied; [ New link ] is the way back. Built.
8. A re-invite keeps the old member id and label; after 180 days, a new member. Built (D:B37, B38).
9. The token flag = a parent proof within 30 days. Built (D:B37).
10. 16-character, 96-bit codes. Built.
11. New copy: the transient join line and the in-app unknown-code and own-Mac owner-note lines (copy table, D:B37, B39 b); a cap-hit line is unwritten.
12. Two live tokens re-place the parent on each entry (H1). Built (D:B37).
13. Own-Mac links are the riskiest artifact; no confirm on the issuing Mac in v1.
14. Instant cut-off is token rotation plus a re-place, docs only.
15. The needs-update line in the invite window. Built (D:B41 n; DV #30).
16. A pending own-Mac row with Cancel…: not built, as recommended (the route exists).
17. The mark on join pages whatever the switch says. Built; open per D:B108.
18. The removed owner-class Mac's face. Built (D:B40 g; DV #21).
19. A broken roster: reseed (dropping every member and invite) or hand repair. Not built; `set_aside` has no caller.
20. The installer's file hand-off (H9). Built (D:B44, B45).
21. Scaffold numbers: 20 uploads and 60 page keys per device-hour (D:B125), the daily stamp, the 30 s debounce, the sweep's 1-day and 7+1-day rule (D:B91).
22. The tab icon ignores the mark switch (D:B109).
