# shroom v1 — the Worker interface (Region 6 and the remaining Worker gaps)

> **Reading it today (2026-10-13):** `build/v1` has been merged into `main` and retired, so
> where this says `build/v1`, read `main`. It is the design record as built: amended in
> place by the DECISIONS rows it names, and every `path:line` citation is as of the commit
> it gives — the named function or line text is the anchor when a line has drifted.
>
> Status: **draft, 2026-10-02**, written on `build/v1` before any Region 6 code, as
> ARCHITECTURE §12 asks ("design as one Worker interface doc before Region 6 code") and
> §16 #4 lists. Synthesized from three competing designs (minimal, security-first,
> spec-coverage), judged twice. The coverage design won. Grafts from the other two and the
> judges' must-fix items are applied throughout. **Revised the same day after a critic's
> review:** its coverage (U1–U9), security (H1–H10), contradiction (C1–C8) and precision
> findings are applied in place; where a finding is declined, the doc says why.
>
> **What this is:** every route, credential, stored record and on-disk file the Worker and
> the app need for members, invites, the join machine and member life (SM:1282-1590). It
> also covers the Worker gaps §8.1 still marks "build call": `?k=` issuance, the space
> identity and version, deregistration, enroll, the token-holder flag, member labels, and
> the token-less Library.
>
> **What this is not:** a decision Bagrat has made. The spec wins every disagreement.
> Spec bends go to `DEVIATIONS.md`, and in-frame calls go to `DECISIONS.md` when built.
> The calls that belong to Bagrat are listed in §1 and asked in §9.
>
> **Standing tags** (as in ARCHITECTURE): **ratified** · **probed** · **recommended** ·
> **build call** · **Bagrat**.
>
> **Citations:**
> - *SM* = UX-STATEMACHINE.md, *A* = ARCHITECTURE.md, *D* = DECISIONS.md,
>   *DV* = DEVIATIONS.md, *R* = RESEARCH.md, *RQ* = the rethink interview (not public), *r6* = spec/r6.json.
> - Code is cited as `path:line` on `build/v1` as of this revision (HEAD `3387c76` plus the
>   working tree); every reference was re-checked then. The tree is moving, so a line
>   number can drift — the named function or line text is the anchor.
>
> **Built since, provisionally (2026-10-03, at `6a623b0`):** this design is built — the
> Worker by DECISIONS B37, the Mac side by B39–B42 — with the review's amendments of B38,
> and later fixes B49 b, B50, B51 d, B53 and B54. Where the build differs from or settles
> what is written below, an **Amended** or **As built** note stands at that place: §2.3,
> §3.5, §4.3, §4.5, §4.8, §4.10, §4.15 (B38), §4.18 (B49, B53 c), §5, §6.9, §6.10, §6.11
> and §9 (which questions were built as recommended). Every §9 question is still
> Bagrat's to answer; nothing here was ratified by building it.

---

## 1. Purpose and standing

*Standing: recommended throughout. The member model (N3/N4/N5) and the Members face (N28)
are ratified. Everything from the join page through member life (N29–N34) is designed but
not yet ratified (r6 note 0). This interface is therefore provisional twice over.*

### 1.1 What it settles (as recommendations)

- **§16 #4:** every route, with its method, path, auth, request/response JSON and error
  classes.
- **§16 #13 / #45:** the `?k=` issuance and the lifecycle of all four secrets.
- **§16 #24:** the page interface version.
- **§16 #44, one face of it:** the owner page's "Waiting for your space" line (§4.18). The
  rest of #44 is named there and in §9 as still open.
- **§16 #46 / #47:** the mint parent, and the complete binding list. #46 is closed only by
  §3.4's re-placement rule (H1): every script PUT, every `space-found`, and every
  `stale-parent` answer to a Mac whose token is live re-places `R2_PARENT`. Without that
  rule a fresh paste leaves the Worker on a dead parent.
- **§16 #57:** the exit recipe's deregistration.
- **§16 #58:** the mint door is chosen by whether this Mac holds the token, and roles live
  in the registry.
- **§16 #59:** the join and invite payloads.
- **§8.1's remaining gaps:** space identity and version, `?k=` issuance, deregistration,
  member labels, enroll, and the token-holder flag.
- **One live security bug — fixed (commit `ff84bb7`, §3.6).** `GET /d/<id>` streamed
  whatever bucket key `meta.media.mp4` named. Once members can write their own
  `meta/<id>.json`, that would have been a read of `.shroom/cookie.key` or the registry.
  It now streams the fixed key `rec/<id>/video.mp4` (`worker/src/worker.js:353-361`), and
  a test pins every key the Worker's read routes may touch.

### 1.2 What stays Bagrat's (asked in §9, with a recommendation each)

| # | Call | This doc's recommendation |
|---|---|---|
| §16 #3 | Where the member registry lives | **R2**, one document behind a two-function interface (§2). The delta for a Durable Object is spelled out. |
| §16 #4b | What the Worker may do for Macs without the token | **Read and mint only**, for recordings. One filtered Library list. Renames and deletes stay on the Mac, under a 15-minute credential the Worker mints. No Worker delete or meta claim. |
| §16 #11 | Who owns a member's recording | The Mac still claims the meta (B15, B21). **Authority moves to a Worker-written, create-only ownership record**, `.shroom/registry/recs/<id>`, written at the first mint (§2.4). `meta.owner` becomes informational. |
| B25 amend | The owner cookie | **A DECISIONS amendment to B25**, logged when built: the cookie becomes `{m, d, t}` (it names the issuing device), the grant becomes `{member, device, exp}`, and the grant's writer becomes `POST /_shroom/api/key` for every Mac. Every use checks the cookie against the registry, so Remove… ends that browser's powers. Token Macs keep B25's direct S3 grant as a fallback, so owner-mode never vanishes on an older page (§3.5). |
| B29 amend | The installer's hand-off | The install script hands the join link to the app **by file** (`~/.shroom/invite`, mode 600), not `--args --join <link>`, so a live join code never sits in the app's argv (§6.12, H9). |

### 1.3 Constraints this design holds to (from the brief and the laws)

- **One dependency-free ESM Worker file, WebCrypto only.** Today's `worker.js` (590
  lines) grows by roughly 600–700. (R7.1)
- **Every write is idempotent and safe to retry.** Callers send ids they generated
  themselves, or hashes of secrets they generated.
- **Strong consistency where the spec says "right away".** A cancelled invite or a removed
  member is refused on the very next request, from any colo. (SM:1366, 1299-1302)
- **Secrets never in argv, logs or URLs** — except law 13's one exception: a join link,
  once, in an install command (SM:153-155). §6.9 and §6.12 remove the two other places
  this build would break it: the `?k=` open (argv of `open`, and the quiet-mode log line
  at `app/src-tauri/src/runtime/cloud.rs:61`), and the installer's
  `open … --args --join <link>` (`installer/install.sh:130`), which would keep a live code
  in the app's argv for the app's whole life.
- **Members never hold the Cloudflare token, the parent pair, or any credential over
  `.shroom/`.**
- **Product voice** in anything a person sees. This document is internal, so it names
  mechanism. The said-lines it routes to are the spec's, quoted. New lines are flagged in
  §9.

---

## 2. The registry

*Standing: **Bagrat** (§16 #3). Recommended: R2.*

### 2.1 Recommendation: R2, one document

The **roster** is `.shroom/registry/roster.json` in the space's private bucket
(`shroom-space`, B24). It holds all three tables: members, devices and invites.

**Every change is one compare-and-swap (CAS)** on that one object:
- read it;
- apply a pure function;
- write it back with `onlyIf: { etagMatches }` (B21, probed live).

Because every invariant lives in one key, three properties follow:
1. **Consume is atomic and single-use.** Marking the invite used, creating or reviving the
   member, and creating the device happen in one write.
2. **Cancel and remove take effect right away.** Each is one write, and every
   authenticated request reads the roster fresh, with no isolate cache.
3. **Retries are safe.** Every mutation is idempotent on caller-made ids.

**Why R2 over a Durable Object:**
1. **The registry survives losing the Worker.** Deleting the Worker is a documented repair
   row: "the page (Worker) or its key gone → Wiring it up → `wiring` runs again — the click
   was the consent; every call is idempotent" (SM:651), and the repair asks nothing else of
   the owner (SM:649-651). A Durable Object's storage goes away with the script that hosts
   its class, which would silently revoke every member and every Mac without the token.
2. **Wiring seeds it over S3** (create-only, `If-None-Match: *`, probed B15), next to
   `cookie.key` and the manifest (`crates/shroom-cloud/src/wire.rs:90,112`).
   `space-found` sees it intact.
3. **No new binding type**, no migration tags on every page deploy, and no Durable Object
   quotas to probe under the two-permission token.
4. **It is the owner's own storage**, visible in their dashboard (OSS trust posture). It
   gets the same backup story as the videos.

**R10's rule.** R10 said the registry must be "Worker-side … never in the public bucket"
(R:2402-2404). That rule was written for a public bucket. Under R6 the bucket is private,
and three facts hold:
- **No route reads `.shroom/registry/`** except the registry module. `/m/` builds only
  `rec/<id>/…` keys (`worker.js:102-108`), and §3.6's read-allowlist test (`ff84bb7`)
  asserts the rest; it grows with every route this doc adds.
- **No device credential ever covers `.shroom/`** (B32 live: `.shroom/` → 403).
- **The roster holds no plaintext secret**: only hashes of device secrets and join codes.
  Labels are admin-private plaintext.

Choosing R2 reopens R10's rule. That is part of the decision.

**KV is rejected.** Its eventual consistency (60 s or more) and cached negative lookups
would let a cancelled invite through.

**The cost of one key (C5).** R2 sustains about one write per second to a single key, and
the roster is one key. Every registry change is a human act (an invite, a join, a remove)
except the token Mac's enroll stamp, so §3.2 throttles that stamp to once a day. P1 (a
hard gate, §7.2) measures what R2 actually answers past the limit, and `mutate` treats
that answer like a lost CAS.

### 2.2 The narrow interface

Nothing in `worker.js` touches registry keys outside this module:

```js
/** @typedef {{ read(): Promise<{roster: Roster, etag: string} | null>,
 *              write(roster: Roster, etag: string): Promise<boolean> }} RegistryStore */
async function roster(env)          // one fresh read per request; never cached across requests
async function mutate(env, fn)      // ≤ 5 tries:
//   r = read(); absent → throw Space("unwired"); unparseable → throw Space("broken")
//   out = fn(structuredClone(r.roster), now) → { roster?, result } | throws a classed error
//   no roster change → return result
//   if (await write(out.roster, r.etag)) return result
//   else sleep(jitter 50–400 ms) and retry; after 5 → 503 {error:"busy"}
async function ledgerGet(env, id)   // .shroom/registry/recs/<id> → {member, device, at} | null
async function ledgerCreate(env, id, rec)  // create-only; on conflict returns the existing record
```

**The R2 store:**
- `read` = `BUCKET.get(KEY)`, parsed as JSON, with `object.etag`.
- `write` = `BUCKET.put(KEY, json, { onlyIf: { etagMatches }, httpMetadata })`, and
  returns `!== null`.
- A failed precondition is treated as "retry" **whether the binding returns `null` or
  throws**, and so is R2's answer to more than one write per second on the key (C5).
  Which shape each takes is unprobed (L2 P1). This is minimal's contract, grafted.
- **P1 is a hard gate (C4).** No Region 6 code is written until two concurrent
  `put(onlyIf:{etagMatches})` calls on one key through the binding are shown to let
  exactly one win. If R2 can't do that, the registry moves to the Durable Object (§2.5),
  and that switch goes to Bagrat before any code.

**The Worker never creates the roster.** A missing roster gives 503
`{error:"space", reason:"unwired"}`; a roster that is present but won't parse gives 503
`{error:"space", reason:"broken"}`. This avoids depending on an unprobed create-only put
through the binding for the one object that matters most. The two reasons let the token
Mac's repair pass tell a space wired before Region 6 (seed it) from a damaged roster
(§6.11's recovery, C8).

**Rust side:** `shroom-cloud::registry::seed`, a create-only S3 `PUT` used by wiring
(§6.11), and `registry::set_aside`, which the token Mac's repair pass uses only on a
`broken` roster (§6.11). Nothing else on the Mac reads or writes the roster. Every other
registry change goes through the Worker, including the token Mac's enrollment (§3.2).

### 2.3 The roster schema (v1; additive only; readers ignore unknown fields)

```jsonc
{
  "v": 1,
  "createdAt": "iso",
  "members": {
    "m1": { "role": "owner", "name": "Bagrat", "state": "active", "joinedAt": "iso" },
    "m_Ab12…(16)": { "role": "member", "label": "Mher", "inviter": "Bagrat",
                     "state": "active" | "removed" | "left",
                     "joinedAt": "iso", "endedAt": null | "iso", "invite": "i_…",
                     "reinvites": [ { "invite": "i_…", "label": "Mher H.", "at": "iso" } ] }
  },
  "devices": {
    "d_Qx9…(16)": { "member": "m1" | "m_…", "name": "MacBook Air" | null,
                    "verifier": "<64 hex = sha256(device secret)>",
                    "state": "active" | "removed" | "left",
                    "joinedAt": "iso", "endedAt": null | "iso",
                    "via": "enroll" | "i_…",
                    "tokenKey": "<parent key id at the last parent proof>" | null,  // informational
                    "proofAt": "iso" | null }  // the token-holder flag reads this alone (§3.2)
  },
  "invites": {
    "i_Kp3…(16)": { "codeHash": "<64 hex = sha256(code)>", "kind": "member" | "own_mac",
                    "label": "Ani" | null, "inviter": "Bagrat", "by": "d_…",
                    "createdAt": "iso", "expiresAt": "iso",
                    "state": "pending" | "used" | "cancelled",
                    "used": null | { "at": "iso", "member": "m_…", "device": "d_…", "verifier": "<hex>" },
                    "endedAt": null | "iso" }
  }
}
```

**Ids:**

| Id | Form | Made by |
|---|---|---|
| `m1` | literal, 2 characters | Member #1, the same id on every owner Mac. `app/src-tauri/src/runtime/cloud.rs:342` already writes `"m1"`. |
| `m_` + 16 base64url (12 CSPRNG bytes), 18 characters | | The Worker, at consume |
| `d_` + 16, 18 characters | | The Mac, before its first call |
| `i_` + 16, 18 characters | | The inviting Mac |

**They are told apart from recording ids by length, never by prefix (C3).** A recording id
(B14) is 16 characters of base64url with no structure, so one in about 4,096 begins with
`m_`, `d_` or `i_`. Every check that matters is shape plus place: recording ids match
`^[A-Za-z0-9_-]{16}$` and live only in `rec/`, `meta/` and the ledger's key; registry ids
match `^[mdi]_[A-Za-z0-9_-]{16}$` (or `m1`) and live only inside the roster and in
`/_shroom/api/{invites,members,devices}/<id>` paths. No code path accepts one where the
other belongs.

**Derived, never stored.** "Expired" is `state == "pending" && now > expiresAt`.
Lifetimes are set by the server's clock:
- member invites: `604800` s (seven days, SM:1328);
- own-Mac invites: `3600` s (one hour, SM:1387).

**Pruning,** inside any `mutate`. A record that still backs something on screen is never
pruned (U7):
- **Pending member invites are never pruned**, expired or not. An expired one is still a
  row in Members ("expired: the row offers [ New link ]", SM:1341-1342) until the owner
  renews or cancels it. Only unexpired ones count toward the pending cap below, so stale
  rows can't block a new invite.
- **Used invites** stay as long as the device they made is in the document (pruned with
  it, below). They back the idempotent replay of a lost join reply and the page's
  `join-used` face.
- **Cancelled invites** go 30 days after `endedAt`. Their row left Members at the cancel.
  After pruning, a late visitor gets the nameless "doesn't work" face instead of
  "expired" — both are terminal faces, and both say to ask for a fresh one.
- **Expired own-Mac invites** go 30 days after `expiresAt`. They have no row in v1 (U4,
  §6.4).
- **Members are never pruned.** Removed and left members keep their label for *Everyone*
  (SM:1369-1371, 1564-1565), and a `previous` revive needs them (§6.3).
- Ended devices are pruned 180 days after `endedAt`. A revoked Mac that comes back after
  that re-joins with a fresh member id; this is named in §9.

**Caps,** checked inside `mutate` (security graft). They bound the document and the
free-plan CPU spent on one `JSON.parse`:

| Cap | Limit |
|---|---|
| Pending, unexpired invites | ≤ 20 |
| All invites in the document | ≤ 500 |
| Active members | ≤ 200 |
| m1 devices (active) | ≤ 20 |
| `label` and Mac `name` | ≤ 64 characters, NFC, control characters stripped |
| `inviter` | ≤ 40 characters, same normalization |

A breach returns 409 `full`. At about 300 B per entry, a 50-person team's roster is about
30 KB.

> **Amended by DECISIONS B38 (built):** the 500-invite cap **evicts before it refuses** — at the cap,
> cancelled invites, expired own-Mac links and used invites whose Mac has ended go early,
> oldest first; pending member invites and used ones behind an active Mac never go.
> `enroll` (a parent proof) is **not** held to the 20-Mac `m1` cap, for a new Mac or a
> removed one, so a fresh token always recovers (§3.7); own-Mac joins stay capped. And
> (B37) an over-long label or inviter is 400 `bad-request`, not 409 `full`; a Mac's sharing
> name is cut to 64, never refused.

**Upload rate (H4) lives outside the roster,** so first mints never write the one hot key:
`.shroom/registry/rate/<did>` = `{"window":"<iso hour>","n":<count>}`, one small object per
device, updated by CAS (`onlyIf: {etagMatches}`, created create-only). §3.4 has the rule.

### 2.4 The ownership ledger (§16 #11)

`.shroom/registry/recs/<id>`:
- body `{ "v":1, "member":"m_…", "device":"d_…", "at":"iso" }`
- `customMetadata { m: "<member>" }` (over S3, the header `x-amz-meta-m`), so a binding
  list with `include:["customMetadata"]` gives the id → member map without a GET per
  entry. A list page holds at most 1,000 entries and may hold fewer with `include`
  (§4.10 pages on `truncated`, never on the count; P3).

**Write rules:**
- **Written create-only, once per recording, before the first byte of that recording
  reaches the bucket.**
  - By the Worker at an id's first upload mint (§4.8).
  - By a token Mac over S3 (`If-None-Match: *`, probed B15, with `x-amz-meta-m`) at its
    first local mint. A failed ledger write stops that upload before its first byte; the
    take waits like any other.
- Never rewritten. Never deleted by any device credential.
- The token Mac's GC sweep may reap entries with neither a meta nor any `rec/` bytes after
  30 days.
- **Immutable, so cacheable forever:** a Mac that has read an id's ledger entry never
  needs to read it again.

**What it replaces.** Mint, `canEdit`, the title and mark checks, and **every** Library's
*Mine · Everyone* filter, row label and "This is Mher's video." — the token Mac's
included (H3, §4.10's `owners` scope) — key on the ledger, never on member-writable
`meta.owner`.
- **Ids with no ledger entry but an existing meta** are member #1's. This covers the
  fenced test spaces made before Region 6, where only token Macs wrote.
- **Ids with neither** belong to whoever mints first.
- `meta.owner` is still filled with the member id, for readability (A §5.3), and is never
  trusted.

**What it closes.** Today's `canEdit` trusts `meta.owner` (`worker.js:186`). Without the
ledger, any member who learns an id while its meta is absent can mint for it and claim it.
Upload mints at capture start (DV #4), so that window is the whole take on an offline
Stop. The ledger is written at the maker's first mint. That leaves one residual window,
named in §3.7: **an offline capture start**, whose link is then shared before the maker
reconnects.

**Cost:** one Class A write per recording (two for a Worker mint, with the device's
budget counter, §3.4). Rejected closer: deriving ids from a device key (security's B14
amendment, §8).

### 2.5 Delta if Bagrat picks a Durable Object

The interface boundary is exactly `read()` + `mutate()`; routes, payloads, auth, app code
and tests are otherwise identical.

1. **Code.** `worker.js` also exports `class Registry` (SQLite-backed). It holds the roster
   as one value in `ctx.storage` and exposes RPC `read()` and `mutate(op, args)`. The pure
   functions run inside the object, so CAS retries and the `busy` class disappear.
2. **Deploy.** The metadata gains:
   - `{type:"durable_object_namespace", name:"REGISTRY", class_name:"Registry"}`;
   - `migrations: {new_tag:"r1", new_sqlite_classes:["Registry"]}` on the first deploy.

   Every later page deploy must send `old_tag`. That is a new wiring and update failure
   mode, with L2 tests to match.
3. **Probes first.** Can *Workers Scripts Edit* alone create a namespace by script upload?
   What are the free plan's limits on SQLite-backed Durable Objects?
4. **Seeding.** Wiring stops seeding `roster.json`. Enroll creates `m1` inside the object.
5. **The ledger** can stay in R2 (recommended) or move into the object.
6. **Loss mode.** If the Worker script is ever deleted, every member and every Mac without
   the token reads `unauthorized`, which is `member-revoked`, and must be re-invited. That
   contradicts SM:651's repair row, where re-running wiring is all a lost page costs
   (SM:649-651), and needs a DEVIATIONS entry. Wiring's repair order also gains "never
   delete-and-recreate the script".
8. **The roster's write-rate limit** (C5) and the `busy` class go away; the parent-proof
   stamp throttle stays harmless.
7. `space-found` and support can no longer inspect members through the bucket.

---

## 3. Auth and credentials

*Standing: recommended. B32's minting is probed. B25's cookie is probed, and amended
here.*

### 3.1 The five ways in

The Worker matches each route to exactly one of these. "None" is a sixth: public pages,
`/_shroom/ping` and `/join/<code>`.

| Credential | Held by | Sent as | Proves | Revoked by |
|---|---|---|---|---|
| **Device key** | every Mac in the space, whatever its role | `Authorization: Bearer <did>.<secret>` | this device is active, and its member is active | Remove… (member or Mac), Leave, Disconnect |
| **Parent proof** | Macs holding the Cloudflare token | `Authorization: ShroomParent <ts>.<sig>` | the caller knows the secret behind the Worker's current mint parent | re-placing `R2_PARENT` with another token's pair (§3.4). Deleting or rolling the token in Cloudflare is not enough on its own: the old secret still proves until a live-token Mac re-places the parent, which its next entry does |
| **Join code** | the invitee, in the link | the path of `/join/<code>`; the **body** of peek and join | the right to consume one pending invite | Cancel invite (member invites); use; expiry |
| **Owner cookie** (B25, amended in DECISIONS when built) | a browser | `shroom_owner` cookie | the issuing device is active; metadata powers only | removing the issuing device or its member |
| **Recording credential** (B32) | the Mac uploading, renaming or deleting one recording | S3 SigV4 with a session token, direct to R2 | R2 enforces `rec/<id>/` + `meta/<id>.json` | expiry: 7 days (upload), 15 minutes (manage) |

### 3.2 Device key and parent proof

**Device key** (coverage's design, with minimal's write order):

**Generation and storage.**
- The Mac generates `did` (`d_` + 12 CSPRNG bytes in base64url) and `secret` (32 CSPRNG
  bytes in base64url, 43 characters) **before** its first call. They live in a file (§5)
  first, so a lost response can be retried with the same values.
- The first call (join or enroll) carries only `verifier = hex(sha256(secret))`. The
  secret first travels as the bearer on later calls: over TLS, to the owner's own Worker,
  never in a URL or argv.
- App logs redact `Authorization`. The Worker never logs headers. It has no `console.*`
  calls on request paths.

**The check.**
1. `devices[did]` exists.
2. `sha256(secret)` equals `verifier`. Comparing hashes leaks nothing through timing.
3. `devices[did].state == "active"`, else 403 `removed` with `why`.
4. `members[device.member].state == "active"`, else 403 `removed` with `why`.

An unknown `did` or a wrong secret gets 401 `unauthorized`.

**Roles** are always read live, as `members[m].role`, and are never taken from the client
or cached.
- **Owner-class** = `role == "owner"` (member `m1`).
- **Member** = `role == "member"`.

**Rejected:** per-request signatures with a device key (security's Ed25519 design, §8).

**Parent proof** (coverage's bootstrap, with domain separation and the clock rule from
the must-fix list):

```
sig = b64url(HMAC-SHA256(key = UTF-8 bytes of the parent's S3 secret,
                         msg = "shroom-parent-v1\n" + METHOD + "\n" + host + "\n" + path
                               + "\n" + ts + "\n" + hex(sha256(body))))
```

- **The key.** "The parent's S3 secret" is the token's derived S3 pair (B32), the same
  key `mint.rs` signs JWTs with (`crates/shroom-cloud/src/mint.rs:49`).
- **Domain separation.** The fixed `shroom-parent-v1\n` prefix means no JWT signing input
  (which always starts `eyJ`) can ever be a valid proof, and the reverse.
- **No secret is sent.**
- **Replay.** A replay within the clock window repeats only an idempotent enroll whose
  body (and so `did` and `verifier`) the signature already binds.

The Worker reads `R2_PARENT` (§3.4) and answers:

| Answer | When | App |
|---|---|---|
| 503 `space` / `no-parent` | the binding is missing | re-place it (§3.4), then enroll again; the repair mood only if that fails |
| 401 `clock` | \|now − ts\| > 300 s | **transient**: correct the offset once from the response's `Date` header, then retry; never a broken space |
| 401 `stale-parent` | the HMAC doesn't verify | the Worker's parent belongs to another token (§16 #46). **A Mac whose token is live re-places `R2_PARENT` with its own pair and enrolls again, once (H1, §3.4).** A Mac whose token is dead goes to the repair mood's `token` row instead |

**What parent proof can do:**
- enroll or revive a device **under `m1` only** (never under a member), including a
  device another owner Mac removed (§4.15, U8);
- stamp `tokenKey` (informational) and `proofAt`;
- refresh `m1.name`.

That is the whole power. It is how member #1 is enrolled at wiring, at `space-found` and
at [ Reconnect here ] / [ Add a token here ] (SM:588, A §8.1 "Enroll"). Re-placing
`R2_PARENT` is not a Worker power at all: it is the token's own Workers power, used from
the Mac.

**The token-holder flag (H1)** is `proofAt` within the last 30 days (open question 9),
and nothing else. A proof verifies only against the Worker's current parent, and every
live-token Mac makes its own pair the current parent before it proves (the `stale-parent`
row above). So the flag reads "this Mac proved a live Cloudflare token within 30 days",
whichever token is placed right now. `tokenKey` is kept for support, never for the flag.
- A Mac without the token can't fake it.
- The app enrolls only after the sweep's verify call has found its token live, so a Mac
  whose token was deleted stops refreshing `proofAt`, and its flag lapses within 30 days.
- **Stamp throttle (C5):** enroll rewrites `proofAt` only when it is more than 24 hours
  old, or when the Mac's `name`, its verifier or `m1.name` changed, or the device is new or
  revived. Every other entry's enroll is a read that answers 200 and writes nothing.

The flag drives:
- "keeps your Cloudflare token" and the missing Remove… (SM:1393-1399);
- the "only token-holder" lines in `join-switch-confirm` (SM:1552-1554) and the Disconnect
  sheet (SM:1213), through `/me`'s `tokenHolders` (§4.7).

### 3.3 Join code and `k`

**Join code.**
- 12 CSPRNG bytes in base64url: **16 characters, 96 bits**, the B14 generator. The
  spec's `7xk2pm` is illustrative, as B14's ids are.
- **The inviting Mac generates it and sends only `codeHash`** (minimal's graft, and must-fix
  5). The Worker first sees the plaintext code at peek or join, and never stores it.
- It rides the `/join/<code>` path (the link itself, law 13's exception) and POST bodies.
  It never appears in a query string.
- The join page sends `Referrer-Policy: no-referrer`, so "made with shroom" can't leak it.
- Single use comes from the CAS. Cancel takes effect on the next request.
- At 96 bits, online guessing is pointless, so no failure path writes a rate counter.

**`k`** (the one-time page key, B25 amended — a DECISIONS amendment when built, C2):
- The Mac generates it: 32 CSPRNG bytes, 43 characters. It lives only inside the
  `OpenOwnerPage` handler, as the existing `owner.rs::grant` does. It is never logged,
  quiet mode included (H7, §6.9).
- The Mac sends `kHash` to `POST /_shroom/api/key`. `kHash` must match `^[0-9a-f]{64}$`
  exactly, else 400 (H5). The Worker writes `.shroom/keys/<kHash>` =
  `{member, device, exp: now + 600 s}` (`KEY_TTL_SECS`,
  `crates/shroom-cloud/src/owner.rs:13`) **create-only** (P2's create-only put). A retry
  after a lost reply finds its own grant (same member and device) and answers 200; any
  other existing grant answers 409 `conflict`, and the Mac makes a fresh `k`.
- **Redeem spends the grant by CAS, then deletes it (H8).** Today's redeem reads, deletes
  and sets the cookie (`worker.js:155-173`), so two concurrent redeems of one `k` (a
  browser prefetch beside the real navigation, a link scanner) could both read the grant
  before either deletes it. The amended redeem reads the grant with its ETag, rewrites it
  as the same grant plus `spent: now` under `onlyIf: {etagMatches}`, and only the writer
  that wins sets a cookie. It then deletes the grant (best-effort). A loser, or a grant
  already `spent`, gets the same silent 302 as an unknown `k`. This rides P1's CAS,
  already a hard gate.
- Grants nobody redeemed (the browser never arrived) are reaped past `exp` by the token
  Mac's sweep (§6.8). Spent grants a failed delete left behind go the same way.
- **Every Mac uses this route**, token Macs included — with one fallback (C2): when `/key`
  answers 404, 405 or anything not shroom-shaped (a page older than iface 2), a token Mac
  writes the grant directly over S3, as `owner.rs::grant` does today, in a shape both
  pages read: `{member:"m1", role:"owner", device:"<did>", exp}`. Token-less Macs have no
  fallback and open the page clean (open question 4). A 503 `unwired`/`broken` gets no
  fallback either: an iface-2 page can't issue a cookie without a roster to check it
  against (§3.5).

### 3.4 Recording credentials: the Worker's mint

**The binding.** `R2_PARENT` is one `secret_text` binding, whose value is
`{"account_id","bucket","key_id","secret"}` (the token's derived S3 pair, B32).

**Placement — three moments, and #46 closes only with all three (H1, C6):**
1. **Every script PUT carries it** in its metadata (wiring, repair, and the page refresh
   that rides an app update). Only token Macs deploy.
2. **`space-found` re-places it.** A fresh paste that finds the space deploys only when
   its bundled page is newer (§6.10), so it re-places the secret explicitly either way,
   then enrolls.
3. **Any `stale-parent` or `no-parent` answer to a Mac whose token is live** (the sweep's
   verify call answered within this entry) re-places it, then enrolls once more. A second
   `stale-parent` right after a re-place is a failure for the repair mood, never a loop.

Re-placing is one call to the script-secrets endpoint
(`PUT …/workers/scripts/<label>/secrets`, `secret_text`) — the token's own Workers
Scripts Edit power, no redeploy. Its fallback is a full script PUT (P5).

**Two live tokens** (two token Macs, each pasted fresh) make the parent follow whichever
Mac entered last. That is harmless: any live token's pair is a valid parent for the same
bucket, and credentials minted under the other stay valid until that token is deleted.
The cost is one secrets call per entry while both are live. When the owner deletes the
old token in Cloudflare (the docs say to), members' in-flight credentials under it die,
re-mint, and work as soon as the parent is a live token's — at once if the live-token
Mac entered last, else at its next entry (members read `space-needs-owner` in between,
debounced per §4.8). The separate `GET /_shroom/parent` route that coverage proposed is
folded into enroll's `stale-parent` answer.

- Rejected: separate `ACCOUNT_ID` and `BUCKET_NAME` plain-text bindings. One binding means
  one placement.
- Rejected: re-placing only on a repair pass (the first draft, to avoid two token Macs
  flip-flopping). It left the Worker on a dead parent after a fresh paste until someone
  ran a repair, kept a deleted token's secret able to prove, and gave a live token Mac a
  "not current" flag and a Remove… a device key can't honor.

**The mint.** The Worker mints exactly as `shroom-cloud::mint::{jwt, mint}` does
(`crates/shroom-cloud/src/mint.rs:34-61`):

```
header  = {"alg":"HS256","typ":"JWT"}
claims  = {"aud":"<account_id>.r2.cloudflarestorage.com", "bucket":…, "exp":…, "iat":…,
           "iss":"<key_id>",
           "paths":{"objectPaths":["meta/<id>.json"], "prefixPaths":["rec/<id>/"]},
           "scope":"object-read-write", "sub":"<account_id>"}
sig     = HMAC-SHA256(key = UTF-8 bytes of secret, b64url(header) + "." + b64url(claims))
cred    = {accessKeyId: key_id, secretAccessKey: hex(sha256(jwt)),
           sessionToken: base64std("jwt/" + jwt)}
```

**Key order (C1).** `mint.rs` builds both objects with `serde_json::json!`, and this
workspace doesn't enable serde_json's `preserve_order` feature (no crate asks for it, and
`Cargo.lock`'s serde_json 1.0.151 carries no `indexmap`), so its maps are `BTreeMap`s and
serialize **alphabetically**, nested objects included: `aud, bucket, exp, iat, iss,
paths{objectPaths, prefixPaths}, scope, sub`; the header is `alg, typ`. The order above is
that order, not the source order of `mint.rs:36-47`.

**Order matters only for byte equality, never for validity.** R2 checks the HMAC over
whatever bytes the JWT carries, so any order mints a working credential. The **golden
vector** — the same inputs and the same expected JWT bytes, asserted in a `mint.rs` unit
test and in a Worker unit test — is what needs the two signers to agree, so the JS port
emits keys in that alphabetical order (a fixed literal, not `JSON.stringify` of an object
built in another order). If the Rust side ever gains `preserve_order`, the vector breaks
loudly, which is the point.

**The request is checked before it reaches the claims (H6).** `rec` must match
`^[A-Za-z0-9_-]{16}$` (B14's shape, the same `ID` the Worker already uses at
`worker.js:53`), else 400 `bad-request`. Only then does the Worker build exactly two
paths, `rec/<rec>/` and `meta/<rec>.json`. Nothing else from the request reaches a JWT.

**Purposes:**

| Purpose | TTL | Who | Rule |
|---|---|---|---|
| `upload` | 604800 s | any device | Ledger absent and meta absent → create the ledger for the caller (it counts toward the device's upload budget, below). Ledger present → `ledger.member` must be the caller's member (a re-mint; no budget). Meta `deleted` → 410. |
| `manage` | 900 s | any device | The meta must exist. `ledger.member` (or `m1` for a legacy id) is the caller's member, **or** the caller is owner-class. |

- **The upload budget (H4).** A device may open at most **20 new recordings in any clock
  hour** (first mints that create a ledger entry). The count lives in
  `.shroom/registry/rate/<did>` (§2.3), bumped by CAS on each first mint. Over the budget
  → 429 `too-many` with `retryAfter`: the app treats it as a transient backoff, and the
  take waits locally, as it would offline. Re-mints of an existing id never count. Token
  Macs mint locally and have no budget (a device key can't stop them anyway). A
  recording a minute is far past real use, and 20 bounds what a leaked member Mac can
  start before its row is removed. The number is a scaffold flag. *Rejected:* counting
  from the ledger itself (a full list per first mint), and a counter inside the roster
  (a write to the one hot key per recording, C5).
- **No credential ever names a `.shroom/` path.** A removed device can never write itself
  back into the registry or reach `cookie.key` (minimal's invariant, asserted in L1).
- **Token Macs keep minting locally** from the derived pair (N4's owner door, chosen by
  token presence, §16 #58).
- **Upload TTL.** Seven days keeps N32's "a video already on its way can still finish"
  true (SM:1365; r6 scaffold flag 2). B32's probe minted 600-second credentials
  (`crates/shroom-cloud/tests/mint_live.rs:57`); that R2 honors a locally signed JWT with
  `exp = iat + 604800` is **unprobed**, and P6 now covers it. If R2 caps it lower, the
  upload mints that cap and re-mints the same `rec` on expiry (allowed: the ledger
  matches). A removed member's re-mint is refused, so their in-flight video could then
  finish only within the shorter cap — N32's "can still finish" would need a DEVIATIONS
  entry, as the spec's own scaffold note warns (SM:1372-1375).

### 3.5 Owner cookie (B25 amended)

> **Amended by DECISIONS B38 (built):** **login CSRF** — a redeem never swaps a live owner-class
> cookie for a member's (a member's own `?k=` link sent to the owner); the grant is spent
> either way and the 302 is the same. The reverse always goes through.

**This amends a logged decision (C2).** B25 records the grant as `{member, role, exp}`
written by the token Mac over S3, and the cookie as `{m, o, t}`. This section and §3.3
change all three: the grant becomes `{member, device, exp}`, its writer becomes
`POST /_shroom/api/key` for every Mac, and the cookie becomes `{m, d, t}`. It lands as a
B25 amendment in `DECISIONS.md` with this build, not as a silent rewrite of a probed
decision (open question 4).

- **Claims become `{m, d, t}`**: member, issuing device, issued-at. The `o` field is
  dropped; owner-class is read live from the roster.
- **The signature is unchanged:** HMAC under `.shroom/cookie.key`. So are the attributes:
  `HttpOnly; Secure; SameSite=Lax`, 400 days.
- **Every use reads the roster fresh**: on every page render, title POST, mark POST, the
  join page's owner view and "You're in". It checks:
  1. the HMAC verifies;
  2. `devices[d]` is active and belongs to `m`;
  3. `members[m]` is active.

  Owner-class = `members[m].role == "owner"`.
- **`canEdit(id)`** = owner-class, or `ledgerMember(id) == m` (§2.4).
- **The mark** is owner-class only (SM:1569-1571).
- **Cookies from before this change** carry no `d` and read as a recipient. This is
  pre-release only, and they are re-keyed by Open page.
- **Owner-mode never vanishes on an older or half-ready page (C2).**
  - *A token Mac against an iface-1 page* (the app updated, its page refresh not yet
    through — offline, or the MTU black hole): `/key` answers 405, so the token Mac writes
    the grant directly (§3.3's fallback) in the shape `{member:"m1", role:"owner",
    device, exp}`. The iface-1 Worker reads `member` and `role` and issues its old
    `{m, o, t}` cookie, as today. Once the page is iface 2 that cookie reads as a
    recipient, and the very next Stop or Open page re-keys it.
  - *An iface-2 page with no roster:* unreachable on the update path, which seeds the
    roster before the iface-2 script goes up (§6.11).
  - *An iface-2 page whose roster is `broken`* (C8): every cookie reads as a recipient,
    because no device can be checked; pages still render. The token Mac is in the repair
    mood by then, and owner-mode comes back with the roster. Trusting cookies unchecked
    while the roster is unreadable would let a removed Mac's browser back in — rejected.
- **Cost:** one R2 read per page view that carries a cookie (owners and members).
  Recipients pay nothing.

### 3.6 Key confinement (P0 — **done**, commit `ff84bb7`)

**The bug, as it was.** `download()` passed `meta.media.mp4` straight to `media()` as a
bucket key. A member holding an upload credential could have written their own
`meta/<id>.json` with `media.mp4 = ".shroom/cookie.key"` (or the roster), then fetched
`/d/<id>`. With the cookie key, they could forge owner-class cookies.

**The fix, landed before any member credential exists:**
- `/d/<id>` streams the fixed key `rec/<id>/video.mp4` (B13), `worker.js:353-361`.
- `meta.media.mp4` is read only as a presence flag.
- `worker/test/worker.test.js` pins it twice: a record naming `.shroom/cookie.key` gets a
  404 with no secret in the body, and a read-allowlist test drives every read route
  (`/<id>`, `?embed=1`, `/d/`, `/m/`, `/_shroom/status/`, `/oembed`) with hostile meta
  fields and asserts every key read is `rec/<id>/…`, `meta/<id>.json`, or one of
  `.shroom/{mark.json, cookie.key, player/hls.min.js}`.

**Still to do, with each new route:** the allowlist test grows to cover it, adding only
the fixed `.shroom/` keys that route owns (`keys/<hash>`, `registry/roster.json`,
`registry/recs/<id>` for the request's own id, `registry/rate/<did>` for the caller's own
device, `health.json`). Nothing else.

### 3.7 What each leaked artifact can and can't do (security graft)

| Leaked | Can | Can't | Ends when |
|---|---|---|---|
| **A member's Mac** (`device.key`, take creds) | as that member: start up to 20 recordings an hour into the owner's bucket (§3.4's budget); list, rename and delete their own videos; key their own pages; report the space's upload health (§4.18) | see any other id, label or member; mint for others' ids; touch `.shroom/`; invite | Remove: the next call gets 403; issued upload creds live ≤ 7 days |
| **A token-less owner Mac** | list every recording and link; delete any; manage members; remove other token-less owner Macs | remove a token Mac; deploy; repair | any owner-class Mac removes it, and its cookies die with it. A fresh token on any Mac is always enough to recover |
| **A token Mac** | everything | — | Cloudflare: roll or delete the token, **then** a live-token Mac re-places `R2_PARENT` (its next entry or a repair, §3.4). Until then the old secret still passes parent proof, so it can still enroll an owner-class device |
| **A member invite link (pending)** | its first consumer joins, labelled as the invitee | — | Cancel; or the real invitee sees `join-used`, loudly (SM:1455), and the owner Removes the row |
| **An own-Mac link (pending)** | **owner-class for an hour**, under an attacker-chosen Mac name | repair or deploy | its hour, or its one use. **No Cancel in v1**: the spec draws no row for a pending own-Mac link (U4, §6.4). Once used it shows as an unexpected Mac under You (SM:1389-1392), and Remove… ends it. **The riskiest artifact; named residual** (open questions 13, 16) |
| **A cookie** | rename videos it may edit; flip the mark (owner-class) | list, delete, mint, learn ids | removing the issuing device or its member |
| **`k`** | one redeem within 10 minutes; a concurrent second redeem loses the CAS (H8) | — | the redeem, or 10 minutes |
| **A recording credential** | write that one recording's bytes and meta (≤ 7 days) | anything else | expiry; instant cut-off = parent rotation (r6 flag 1) |
| **`R2_PARENT` (the Worker secret)** | every bucket the token's R2 power reaches (B32's caveat) | Workers or API calls | rolling the token and re-placing the parent. The Worker never serializes `env`, and an L1 test asserts no response body or header carries its bytes |
| **A member's false health report** (§4.18) | flip a stalled owner page's line between "Waiting for connection" and "Waiting for your space" | anything else: the report changes one advisory record, never a credential, a meta or the roster | the next true report or enroll |

**Residual windows, stated** (§16 #11, must-fix 3):
- **An offline capture start.** The ledger is written at the first mint. If both the
  capture start and the Stop happen offline, the link exists before the ledger does. A
  teammate who gets that link out of band before the maker reconnects can mint first and
  own it. The maker's take then reads 403 `not-yours`: a logged anomaly, with the take
  kept locally (law 2).
- **A removed member's upload credential** can rewrite their own tombstoned meta until it
  expires. The tombstone-before-resume check (A §5.5) and 410-on-mint close the honest
  path. The token Mac's sweep reaps late bytes.

### 3.8 Cross-site and abuse limits

- **Origin.** Any `/_shroom/api/*` request carrying an `Origin` header is refused with 403
  `origin` (minimal's graft). The app's Rust client sends none. Browsers always send one
  on a cross-origin POST, and the Worker sends no CORS headers. The cookie POSTs keep
  their same-origin check (`worker.js:259-266`).
- **Bodies.** JSON only, ≤ 16 KiB (≤ 128 KiB for `library`), else 413.
- **Failure paths are read-only.** Unauthenticated routes (join page, peek, join) never
  write on failure, and each does at most one roster read.
- **No rate counters on unauthenticated paths.** The 100k/day free-plan cap stays the
  outer bound for a flood (A §8.3). A 429 from Cloudflare is transient everywhere in the
  app. The one counter is authenticated: the per-device upload budget (§3.4, H4).
- **Everything a person typed is escaped where it renders (H10).** The inviter's short
  name and the invite's label reach HTML on the join page — inside text nodes and inside
  `<title>` — so both go through `escapeText` (`worker.js:474`), and any attribute use
  through `escapeHtml` (`worker.js:480`). The inviter comes from whichever owner-class Mac
  made the invite, so a leaked token-less owner Mac could choose it. Unescaped, it would
  be script on the origin that holds every owner cookie.

---

## 4. Routes

*Standing: recommended; paths are build calls settled here.*

**Error bodies.** Every `/_shroom/api/*` error is `{"shroom":1,"error":"<class>"}`, plus
`reason` or `why` where noted. Success bodies carry `"shroom":1`.

**The shape rule (U2).** The app treats an answer as the space's own only when its body is
JSON carrying `"shroom":1`. Anything else — an HTML 404 from workers.dev, another
Worker's JSON, a Cloudflare error page, an iface-1 page's empty 405 — is **not a class**:
the app re-pings (§4.2) and routes on the ping. Not shroom → `join-space-missing` (or
`space-gone` once joined, debounced per §6.7); shroom but `iface < IFACE_MIN` → the
needs-update face (§6.10); unanswered → `join-waiting` during a join, a silent backoff
once joined. So a status code alone never maps to a state: a 404 is
`join-expired` only when it is `{"shroom":1,"error":"unknown"}`. (Ping keeps its
existing `"shroom":true`; `cloud.rs:49` already matches that exact text.)

**Versioning.** The interface is `iface: 2`; today's ping says 1 (`worker.js:96`). Routes
and fields only grow within an iface (§6.10).

### 4.1 Summary table

| # | Method | Path | Who | Serves (SM states / facts) |
|---|---|---|---|---|
| 1 | GET | `/_shroom/ping` *(amended)* | none | `join-space-missing`, `space-gone`, the stop's reachability, page version (#24) |
| 2 | GET, HEAD | `/join/<code>` | none (+ cookie) | `join-landing`, owner view, `-elsewhere`, own-Mac variant, `join-expired · page`, `join-used · page` (+ "You're in") |
| 3 | POST | `/_shroom/api/peek` | join code | `member-welcome` (+ own-Mac), the `invite` file, the arrival classification, the tray click's one-time `join-expired` |
| 4 | POST | `/_shroom/api/join` | join code (+ the new device's verifier) | `joining` → the ticked row; `join-expired`, `join-used`, `join-waiting`, `join-already-here`, and (through the re-ping) `join-space-missing`; re-invite after removal |
| 5 | POST | `/_shroom/api/enroll` | parent proof | wiring step 10, `space-found`, Reconnect / Add a token, the token-holder flag, the #46 probe |
| 6 | GET | `/_shroom/api/me` | device key | law 1's re-probe; `join-already-here` vs a dead identity; `member-revoked` (and the removed owner Mac's face, U8); the only-token-holder lines; Settings → Space facts |
| 7 | POST | `/_shroom/api/mint` | device key | N4's member door; `member-revoked` / `space-needs-owner`; tombstone-before-resume; Library manage |
| 8 | POST | `/_shroom/api/key` | device key | `?k=` for every Mac (§16 #13): Stop, Get link, Open page, the deferred open |
| 9 | POST | `/_shroom/api/library` | device key | Library for members and token-less owner Macs (N35, §16 #4b, #50); the ledger's id → member map for every owner-class Mac (`owners` scope, H3) |
| 10 | GET | `/_shroom/api/members` | device key, owner-class | `invite-born`'s rows and facts; You sub-rows; every member's label for the Library (H3) |
| 11 | POST | `/_shroom/api/invites` | device key, owner-class | Invite / Return; [ New link ]; Add another Mac |
| 12 | DELETE | `/_shroom/api/invites/<iid>` | device key, owner-class | `member-remove-confirm · a pending invite` |
| 13 | DELETE | `/_shroom/api/members/<mid>` | device key, owner-class | `member-remove-confirm · a member` |
| 14 | DELETE | `/_shroom/api/devices/<did \| self>` | device key: owner-class for `<did>`; anyone for `self` | `member-remove-confirm · your other Mac`; Leave / Disconnect / join-switch (§16 #57) |
| 15 | POST | `/_shroom/api/mark` | device key, owner-class | Settings → Space's mark switch on Macs without the token (N36) |
| 16 | GET | `/<id>?k=<k>` *(amended)* | `k` | owner-mode by role for every Mac class (U2, N40) |
| 17 | GET / POST | `/<id>`, `/_shroom/title/<id>`, `/_shroom/mark` *(amended)* | cookie, checked live | member owner-mode = title only; per-device revocation (#45); the upload line's waiting words (§4.18) |
| 18 | GET | `/d/<id>` *(fixed, `ff84bb7`)* | none | Download (N26) without the key-injection read |
| 19 | POST | `/_shroom/api/health` | device key | "Waiting for your space" / "Waiting for Bagrat's space" on the owner page (N40, SM:946-949; §16 #44) |
| 20 | GET, HEAD | `/favicon.svg`, `/favicon.png`, `/favicon.ico` *(added, B109)* | none | the tab icon of every page the Worker renders — the watch page's faces and the join page's (§4.19) |

**Count.** 15 new routes (2–15, 19) and 4 amended (1, 16–18). Unchanged: `/m/`,
`/_shroom/hls.js`, `/_shroom/status/<id>`, `/oembed`, `?embed=1`. Added since, outside
Region 6: the tab icon's three addresses (20).

**What the Worker gains** is reading (the roster, the ledger, one filtered list), minting,
and writes to its own records only: the roster, the ledger, and three small ones
(`.shroom/keys/<hash>`, `.shroom/registry/rate/<did>`, `.shroom/health.json`). It gains
**no** delete of bytes and no meta claim (open question 2).

### 4.2 `GET /_shroom/ping` (amended)

- **Request:** none. **No storage read**, so it stays inside the stop probe's 600 ms
  (DV #8).
- **Response:** `200 {"shroom":true,"iface":2,"version":"<semver of the app that deployed
  this file>"}`, `cache-control: no-store`. `VERSION` is a constant stamped into
  `worker.js` at build.
- **Errors (classified by the app):**
  - anything but this JSON (a workers.dev 404, another Worker, a Cloudflare error page),
    or NXDOMAIN while the Mac is online (L2 P7) → **not a shroom space**;
  - a transport failure while offline → unanswered.
- **Serves:**
  - `join-space-missing` (SM:1541) and `space-gone` (SM:1580);
  - `iface < IFACE_MIN` → the spec's needs-update lines (SM:1266-1270), on a joined Mac
    **and during a join** (U3, §6.2);
  - the re-ping behind §4's shape rule: every non-shroom answer from another route lands
    here before it becomes a state;
  - a token Mac never deploys over a newer `version` (SM:1754-1756).

### 4.3 `GET|HEAD /join/<code>`

> **Amended by DECISIONS B38 (built):** a used own-Mac link says “You're in” only to a cookie from
> the Mac it let in (every owner-class cookie is `m1`, so the member is no signal), and
> “This link was already used — make a fresh one from Members on your other Mac.” to
> everyone else. The cookie faces are best-effort — DEVIATIONS #31 (§9 Q5).

- **Request:** `code` matches `^[A-Za-z0-9_-]{16}$`; anything else gets the unknown face.
  The Worker reads `User-Agent` and `Cookie`. It does one roster read, plus the cookie's
  check. **Opening never consumes** (SM:1427).
- **Response:** 200 `text/html` for **every** face, so chat unfurlers always see a hero.
  - Headers: `x-robots-tag: noindex`, `cache-control: no-store`,
    `referrer-policy: no-referrer`, CSP `frame-ancestors 'none'`.
  - `<title>` = the face's hero line, escaped (§3.8, H10).
  - The links on the page:
    - [ Open shroom ] = `shroom://join/<host>/<code>`.
    - The install line =
      `curl -fsSL https://shroom.am/install | sh -s -- --join <host>/join/<code>` with
      [ Copy ].
  - **The mark (U6).** Every face carries "made with shroom" + the mark bottom-right, the
    page's only shroom chrome (SM:1439-1440), **whatever the space's mark switch says**.
    The switch's own line scopes it to video pages — "The small mark on every video
    page" (SM:1198) — and the join page is the invitee's introduction to shroom, not
    a video page. So the route never reads `mark.json`, and no face carries the owner's
    switch (the owner view adds its note and nothing else). Bagrat may prefer the switch
    to govern join pages too (open question 17).

| Face (r6 state) | When | Words (SM) |
|---|---|---|
| `join-landing` | pending, unexpired, Mac UA | SM:1429-1440 |
| `join-landing · own-Mac variant` | `kind == "own_mac"` | SM:1457-1461 |
| `join-landing · owner view` | a cookie whose member is owner-class, checked live (§3.5) — *best-effort* | + "This is the page Ani will see — opening it here doesn't use it up." (`label` from the invite, escaped; own-Mac uses the variant's words with the same note, minus the name — new copy, open question 11(b)) |
| `join-landing-elsewhere` | non-Mac UA (`iPhone\|iPad\|Android\|Windows\|Linux\|CrOS`). An inline script also flips a Macintosh UA with `maxTouchPoints > 1` (iPadOS) — *best-effort* | SM:1453 |
| `join-expired · page` | expired or cancelled | "This invite has expired — ask Bagrat for a fresh one." (own-Mac: SM:1460-1461) |
| `join-expired · page`, nameless | unknown or malformed code | "This invite link doesn't work — ask whoever sent it for a fresh one." |
| `join-used · page` | used | "This invite was already used. If that wasn't you, ask Bagrat for a fresh one." |
| `join-used · page`, "You're in" | used, and the cookie's `m == used.member` | "You're in — this invite already did its job. The mushroom in your menu bar is all you need." *Best-effort (§16 #54, open question 5)* |

- **Never returns** the label to anyone but an owner-class cookie.
- **Best-effort faces, named (U6).** Three faces rest on what the browser happens to
  carry, so the page can't promise them:
  - *the owner view* needs an owner-class cookie in this browser — one opened through
    Stop or Open page on this space (a fresh browser shows the plain landing, and the
    owner's click still doesn't use the invite);
  - *"You're in"* needs the joiner's own cookie, which a fresh joiner gets only once a
    page of theirs has been opened keyed (§16 #54, open question 5); without it they see
    the plain `join-used` face;
  - *elsewhere* rests on the user agent: a phone that asks for the desktop site sees the
    Mac face (harmless — the install line just won't run there).

  Every other face is decided by the roster alone and is exact.
- **Errors.** A missing or unreadable roster, or an R2 binding error, renders a
  **transient** line: "This invite link doesn't work right now — try again a little
  later." *(New copy, open question 11.)* It never renders the terminal "ask for a fresh
  one" face (must-fix 4). Other methods get 405.

### 4.4 `POST /_shroom/api/peek`

- **Request:** `{"code":"<16>"}`. The code rides the body, never a URL.
- **Response:** `200 {"shroom":1,"state":"pending"|"expired"|"cancelled"|"used",
  "kind":"member"|"own_mac","inviter":"Bagrat","expiresAt":"iso","now":"iso","iface":2}`.
  It never writes and never returns the label.
- **Errors:**
  - 404 `{"shroom":1,"error":"unknown"}` → `join-expired` (the nameless wording, open
    question 11(b));
  - 400 `bad-request`;
  - 403 `origin`;
  - 503 `space` (`unwired` | `broken` | `storage`) → `join-waiting`'s "isn't answering",
    auto-retry;
  - **anything not shroom-shaped** (§4's shape rule, U2) → re-ping: not shroom →
    `join-space-missing`; `iface < IFACE_MIN` → the needs-update face (§6.10, U3);
    unanswered → `join-waiting`.
- **Serves:**
  - `member-welcome` and its own-Mac variant, which need the inviter's short name and the
    kind before Join (SM:1484, 1500-1503);
  - the `invite` file's `inviter`, `own_mac` and `expires` fields (A §3.4), behind the tray
    rule "reopen while it lives, `join-expired` once after" (SM:1497-1499);
  - the arrival classification (§6.2).

Rejected: `GET /join/<code>` with `Accept: application/json` (minimal). It duplicates
peek, and peek keeps the code out of the request line.

### 4.5 `POST /_shroom/api/join`

> **Amended by DECISIONS B38 (built):** a re-invited member's row reads the re-join date — the
> revive sets `joinedAt`, and the first join date moves into the `reinvites` entry as
> `joinedBefore`.

**Request:**

```jsonc
{ "code": "<16>",
  "device": { "id": "d_<16>", "verifier": "<64 hex>", "name": "MacBook Air" | null },
  "previous": null | { "id": "d_<old>", "secret": "<old secret>" } }
```

- `name` is the Mac's sharing name. It is kept only for `own_mac`, as the sub-row label
  (SM:1402-1404).
- `previous` is sent only by a Mac whose stored identity **for this same host** reads dead
  (`/me` → 401 or 403).

**One CAS:**
1. **Find the invite** by `codeHash`.
   - None → 404 `unknown`.
   - `cancelled` → 410 `cancelled`.
   - Pending past `expiresAt` → 410 `expired`.
   - `used`:
     - same `device.id` and the same `verifier` → **the same 200 again** (idempotent
       after a lost reply);
     - anything else → 409 `used`.
2. **`previous` given:**
   - Verify `sha256(previous.secret) == devices[previous.id].verifier`.
   - If that device is **active** → 409 `already-here`, with nothing consumed (security's
     backstop behind the app's own check).
   - If it is `removed` or `left`, its member is **not `m1`**, that member's state is
     `removed` or `left`, and the invite's kind is `member` → **revive that member id**:
     state active, `endedAt = null`, **the old `label` kept** (H2). The invite's label is
     appended to the member's `reinvites` history (`{invite, label, at}`) and changes
     nothing anyone sees.
     - *Why the old label stays:* a revive proves only that the joiner once held that
       member's device, not that the owner meant this invite for them. A removed Mher
       holding a link meant for Ani would otherwise relabel every one of Mher's old videos
       "Ani" in *Everyone*. Kept, the owner's pending "Ani" row turns into "Mher — joined
       Oct 2": an honest signal of who actually came back (open question 8).
     - *Rejected:* an owner-marked "re-invite" flag on the invite (a new control in the
       composer for a rare case).
   - Any other `previous` is ignored. It never revives `m1`, never touches an active
     member, and never moves a device across members (must-fix 5).
   - The old secret is useless by then: a dead device can't authenticate.
3. **Pick the member:**
   - `kind: member` → a new `m_…` from the Worker's CSPRNG (unless revived), with
     `{role:"member", label, inviter, joinedAt, invite}`.
   - `kind: own_mac` → `m1`.
4. **Add the device.** If `devices[device.id]` already exists → 409 `conflict` (the Mac
   regenerates its `did` and retries). Otherwise add
   `{member, name (own_mac only), verifier, state:"active", via: iid, tokenKey:null}`.
5. **Mark the invite used:** `invite.state = "used"`,
   `used = {at, member, device, verifier}`. Then apply the caps.

**Response:**

```jsonc
200 { "shroom":1, "kind":"member"|"own_mac", "role":"member"|"owner",
      "member":"m_…"|"m1", "device":"d_…", "inviter":"Bagrat",
      "space": { "host":"v.myshroom.workers.dev", "account_id":"…", "bucket":"shroom-space",
                 "label":"v", "subdomain":"myshroom", "owner_name":"Bagrat", "iface":2 } }
```

- `owner_name` is the invite's `inviter`.
- The member never receives its own label (admin-private, N28).

**Errors → app state:**

| Answer | App state |
|---|---|
| 404 `unknown`, 410 `expired` / `cancelled` — shroom-shaped | `join-expired` |
| 409 `used` | `join-used` |
| 409 `already-here` | `join-already-here`, worded by this Mac's role (U5, §6.2) |
| 409 `conflict` | regenerate the `did`, retry |
| 409 `full` | the composer-side cap (§2.3); unreachable from join in practice |
| 400 | — |
| 403 `origin` | — |
| 503 `busy` / `space`, 429, transport | `join-waiting`, auto-resume with **the identical body** (SM:1538) |
| **anything not shroom-shaped** — any status (U2) | re-ping: not shroom → `join-space-missing` (SM:1541); `iface < IFACE_MIN` → the needs-update face (U3, §6.10); unanswered → `join-waiting` |

The same table classifies the **boot replay of `join.json`** (§5): a Mac that crashed
mid-join replays the identical body at launch and lands on whichever state the answer
names, `join-space-missing` included. Only a shroom-shaped terminal answer (404, 410,
409 `used`/`already-here`) or a `join-space-missing` re-ping clears `join.json`.

**Serves:**
- N30's "the Mac's member identity lands locally" (SM:1514-1518);
- the row ticking with the fact "✓ Bagrat's space — `v.myshroom.workers.dev`" (needs
  `host` and `owner_name`);
- the own-Mac variant, where role owner → three doors at `all-set`;
- one-time use (N3);
- re-join after removal, so the waiting takes drain (SM:1588-1589);
- §16 #59 (inviter, address, role, member and device ids).

### 4.6 `POST /_shroom/api/enroll`

- **Request:** parent proof, with
  `{"device":{"id":"d_…","verifier":"<hex>","name":"MacBook Pro"},"ownerName":"Bagrat"}`.
- **Response:** `200 {"shroom":1,"member":"m1","device":"d_…","iface":2,"token":true}`.

**One CAS, written only when something changed (C5):**
- `m1` is created if absent (`{role:"owner", name}`), or its `name` is refreshed when it
  differs.
- `devices[id]` is created or revived **under `m1` only** — including a device another
  owner Mac removed (U8: a token proves more than a Remove… can take away) — with
  `{verifier, state:"active", via:"enroll", tokenKey: R2_PARENT.key_id, proofAt: now}`.
- An existing active device with **the same verifier** gets its stamp refreshed **only if
  `proofAt` is more than 24 hours old** or its `name` changed. Otherwise the call writes
  nothing and answers the same 200: the roster's single key sees at most one stamp per
  token Mac per day.
- A different verifier, or a device that belongs to a member, gets 409 `conflict`.

**Errors:**
- 401 `clock` → transient, with the offset corrected once;
- 401 `stale-parent` → this Mac's token isn't the Worker's parent. **If the sweep found
  the token live this entry, the Mac re-places `R2_PARENT` with its own pair (§3.4, H1)
  and enrolls once more**; a second `stale-parent` → the repair mood. A dead token never
  re-places — it is the repair mood's `token` row;
- 409 `conflict`;
- 503 `space`:
  - `no-parent` → re-place, then enroll again (as `stale-parent`);
  - `unwired` → seed the roster create-only (§6.11), then enroll again;
  - `broken` → the repair pass's roster recovery (§6.11, C8);
  - `storage` → the repair mood.

**Serves:**
- wiring step 10 (N3's silent member #1);
- `space-found`'s "…This Mac is joined." (SM:588, 1597-1598), after its re-place;
- [ Reconnect here ] / [ Add a token here ];
- the stamp on a token Mac's entry, so the only-token-holder count stays right after a
  re-paste (must-fix 6), throttled to once a day;
- §16 #46's probe and its cure (the re-place).

### 4.7 `GET /_shroom/api/me`

- **Request:** device key.

**Response:**

```jsonc
200 { "shroom":1,
      "you":   { "member":"m_…"|"m1", "device":"d_…", "role":"member"|"owner", "name":"…"|null },
      "space": { "host":"…", "account_id":"…", "bucket":"shroom-space", "owner_name":"Bagrat", "iface":2 },
      "owner": { "mark":true, "tokenHolders":1, "token":false } }   // "owner" for role owner only
```

- `owner_name` is the member record's `inviter` for a member, and `m1.name` for
  owner-class.
- `tokenHolders` = the active `m1` devices whose `proofAt` is within 30 days (§3.2's flag),
  this one included when it qualifies.

**What `tokenHolders` can and can't say (D).** The count lags reality by up to 30 days in
one direction only:
- a token Mac whose token was deleted keeps counting until its `proofAt` ages out, so a
  caller may be told it is *not* the only holder when, in practice, it is;
- a Mac that holds a live token always counts (every entry proves, at least daily).

So the app shows the only-token-holder lines when `tokenHolders <= 1` with this Mac
counted, and also **whenever the count can't be had** (the old space's `/me` is
unreachable, U9). Both lines (SM:1213, 1552-1554) are true for any token Mac whose space
still exists — the "only" just decides when they're worth saying — so erring toward
showing them costs a redundant sentence, while erring toward hiding them could cost the
way back. When the old space answers as not-shroom (gone), the join-switch line is
dropped: "everything will be there" would be false.

**Errors:**
- 401 `unauthorized` (identity unknown, e.g. storage set up fresh, or a reseeded roster)
  and 403 `removed` (`why: removed|left`) → `member-revoked` on a member's Mac; on an
  owner-class Mac, the removed-Mac face (U8, §6.7);
- 503 `space` → `space-needs-owner` — **debounced** like mint's (C7, §4.8);
- not shroom-shaped → re-ping (§4's shape rule);
- transport → keep the cached identity.

**Serves:**
- law 1's re-probe on every entry (`member-revoked` clears by itself once re-invited);
- `join-already-here` vs a dead identity (SM:1542, 1588);
- the only-token-holder lines (SM:1213, 1552-1554);
- Settings → Space facts and the owner-class mark switch state (N36).

### 4.8 `POST /_shroom/api/mint`

> **Amended by DECISIONS B125 (built):** the residual below is closed. After a delete, the Mac
> that made it seals it: `POST /_shroom/api/gone {"rec"}` (the maker or owner-class, only once
> the record reads as a tombstone or is gone; idempotent) writes `.shroom/gone/<id>`, which no
> recording credential reaches; a token Mac writes the same key with its own pair. Every read
> of a record (the page, its status, its media, the download, mint) honors it first: a sealed
> video is removed whatever its meta says now, and every mint on it but an owner-class
> `manage` is 410 `deleted`. The upload credential's seven days stand (N32).

> **Amended by DECISIONS B38 (built):** a tombstone is final for its maker — a `manage` mint on a
> deleted record is 410 `deleted` unless the caller is owner-class (not only `upload`
> mints). The residual: an upload credential already issued for that id can rewrite the
> tombstone until it expires (≤ 7 days); a Worker-held deletion record that closes it is
> new design, for Bagrat.

- **Request:** device key, with `{"rec":"<16-char id>","purpose":"upload"|"manage"}`.

**Response:**

```jsonc
200 { "shroom":1, "accessKeyId":"<key id>", "secretAccessKey":"<64 hex>",
      "sessionToken":"<base64>", "expiresAt":"iso", "bucket":"shroom-space",
      "endpoint":"https://<account>.r2.cloudflarestorage.com",
      "owner":"<ledger.member>" }
```

- **`owner` is the ledger's member (C4)** — the entry this mint just created or read, or
  `m1` for a legacy id. The Mac writes it into `meta.owner` when it claims the meta,
  for readability only (§2.4); nothing ever reads authority back from there.

The rules are §3.4's.
- **The request is checked first (H6):** `rec` must match `^[A-Za-z0-9_-]{16}$`, else
  400 `bad-request`, before anything reaches the ledger key or the JWT's two paths.
- **`upload`, first mint, in this order:** bump the device's upload budget (§3.4, H4);
  create the ledger record create-only (on a conflict, re-read: the same member is fine,
  otherwise 403 `not-yours`); only then read `R2_PARENT` and mint. A `no-parent` answer
  therefore still leaves the video's owner recorded, which the owner page's waiting line
  needs (§4.18).
- **Every mint:** one roster read, at most one ledger read or create, and one meta read;
  a first upload mint adds one budget read and write.

**Errors → app state:**

| Answer | App state |
|---|---|
| 401 `unauthorized` / 403 `removed` | `member-revoked` on a member's Mac (Start greyed, SM:1578); the removed-Mac face on an owner-class Mac (U8, §6.7) |
| 403 `not-yours` | a logged anomaly; never re-mint that id; the take is kept |
| 410 `deleted` | tombstone-before-resume: the take stops, its row reads "Removed from the space" (A §5.5) |
| 404 `absent` | `manage` only |
| 400 `bad-request` | a bug: logged, never retried |
| 503 `space` (`no-parent` \| `storage` \| `unwired` \| `broken`) | `space-needs-owner` (Start live; takes drain later, SM:1579) — **after the debounce below** |
| 503 `busy`, 429 (`too-many` or Cloudflare's), transport | silent backoff |
| not shroom-shaped | re-ping (§4's shape rule): not shroom → `space-gone`; else silent backoff |

A credential that is **fresh** but refused on its first PUT also classifies as space-side:
`InvalidAccessKeyId`, `SignatureDoesNotMatch` or `AccessDenied` (a stale parent, a
revoked key, storage switched off). The Mac then also reports it (`POST /health`, §4.18),
so the owner page can say why the video waits.

**Debounce (C7).** One space-class answer is not yet a face: R2 has transient errors, and
a banner that flashes on a blip teaches people to ignore it. The Mac shows
`space-needs-owner` only after **two space-class answers at least 30 seconds apart**
(the second from a re-mint it schedules itself), with no successful mint or PUT between
them. Until then the take waits silently, exactly as it would on a backoff. One success
clears the count, and the face clears by itself (law 1). The removed classes (401/403)
are not debounced: they come from the roster, which has no transient wrong answer.

### 4.9 `POST /_shroom/api/key`

> **Amended by DECISIONS B125 (built):** a device asks for at most 60 page keys a clock hour
> (429 `too-many`, `retryAfter`), counted as `k` in the same `.shroom/registry/rate/<did>`
> object as the upload budget's `n`.

- **Request:** device key, with `{"kHash":"<64 hex = sha256(k)>"}`. `kHash` must match
  `^[0-9a-f]{64}$` exactly — lower-case hex, nothing else — or 400 `bad-request` (H5).
- **Response:** `200 {"shroom":1,"exp":"iso"}`. The Worker writes
  `.shroom/keys/<kHash>` = `{"member","device","exp": now + 600 s}` **create-only**
  (P2). An existing grant for the same member and device is a replay → the same 200; any
  other existing grant → 409 `conflict` (the Mac makes a fresh `k`). Grants left
  unredeemed are reaped past `exp` by the token Mac's sweep (§6.8).
- **Errors:**
  - 401 / 403 `removed` → the page opens **clean**, as a recipient;
  - 404 / 405 / not shroom-shaped (a page older than iface 2) → **token Macs fall back**
    to the direct S3 grant (§3.3, C2); token-less Macs open clean;
  - 503 (`busy`, `storage`, `unwired`, `broken`) → clean;
  - transport → the deferred open stays the core's job.
- **Serves:**
  - B25's open item, "members' grants pending";
  - §16 #13;
  - N40's member owner-mode (title only);
  - the cookie that powers the join page's owner view and "You're in".

### 4.10 `POST /_shroom/api/library`

> **Amended by DECISIONS B38 (built):** the Library never invents an end — a listing still
> truncated after five empty pages is 503 `busy` (a silent backoff), never `next:null`;
> ledger entries without `customMetadata.m` cost at most 40 parallel GETs a call, the page
> ending at the last one resolved. Ownership is the ledger's alone (B54 f).

**Request:** device key, with

```jsonc
{ "scope": "mine" | "everyone" | "owners",
  "known": { "<id>": "<etag>", … }, "knownThrough": null | "<id>",
  "after": null | "<id>" }
```

- Members are forced to `mine`. Asking for `everyone` or `owners` → 403 `forbidden`.
- `known` holds the caller's cached ETags for at most its next 1,000 cached ids after
  `after`; `knownThrough` names the last of them (null when the cache has no more). The
  Worker never compares past `knownThrough`, so every id it judges falls inside the slice
  the client sent, and the body stays under §3.8's 128 KiB however large the Library.

**One page per call, walked in id order (D).** The ledger
(`.shroom/registry/recs/<id>`) and the metas (`meta/<id>.json`) sort by the same id, so
the Worker walks them in lockstep, as a merge-join, instead of listing either whole:
1. List the ledger from `startAfter: recs/<after>` with `include:["customMetadata"]`, and
   `meta/` from `startAfter: meta/<after>.json`, at most 1,000 each.
2. **Never trust the count.** With `include`, R2 may return fewer than the limit and say
   `truncated: true` (P3). The page ends at the smallest of: the last id of each list
   that came back `truncated` (a list that didn't is exhausted and bounds nothing), and
   `knownThrough`. `next` is that end — or null when neither list was truncated and the
   page didn't stop at `knownThrough`. Only `truncated` and the last key decide this,
   never "fewer than 1,000 came back".
3. Keep the ids in scope:
   - `mine` = ledger member == caller's member, or legacy ids (meta, no ledger) for `m1`;
   - `everyone` = all.
4. GET bodies **only for ETags that differ from `known`, at most 40 per call**. If more
   than 40 changed, the page is cut after the 40th and `next` moves back to it. The bound
   is set by L2 P8's per-invocation ceiling and tuned once that probe answers.

`owners` is the ledger walk alone (step 1's first list, no metas, no bodies): it returns
the id → member map a page at a time, **uncapped across pages** (H3).

**Cost.** A warm refresh of n recordings is ⌈n/1,000⌉ calls of two list operations each.
A cold one adds n GETs, 40 per call: about 25 calls at 1,000 videos, fine as a
background refresh. A cold `owners` pass is ⌈n/1,000⌉ calls of one list each.

**Response:**

```jsonc
200 { "shroom":1,
      "items":[{ "id":"…", "etag":"…", "owner":"m_…"|"m1", "meta":{…} /* only when changed */ }],
      "owners": { "<id>":"m_…"|"m1" },   // scope "owners" only, instead of items
      "next": null | "<id>" }
```

- The client computes "gone" as `known` minus every id it saw, once `next` is null.
- Tombstones come back with `state: "deleted"`: they are dropped from counts, and the
  local take's row becomes "Removed from the space".
- **Labels are not here.** Every owner-class Mac reads them from `/members`'s `labels`
  map (§4.11), which keeps removed and left members.

**Errors:**
- 401 / 403 → the window's cached list, with only Copy link and Save to Movies live;
- 403 `forbidden`;
- 413;
- 503 → offline-like (the cached list).

**Serves:**
- N35 for members (their own only, with N5 enforced server-side) and for token-less owner
  Macs (*Mine · Everyone*, "once someone else has a video here" = any item whose
  `owner != "m1"`);
- with `/members`' labels, the label on others' rows and "This is Mher's video.";
- §16 #50's row source;
- posters through the existing `/m/<id>/poster.jpg`.

**Token Macs (H3).** They keep their direct S3 list (`shroom-upload::library::refresh`,
`crates/shroom-upload/src/library.rs:72`) for titles, states and ETags — it also works in
the repair mood — but **never take ownership from it**: `meta.owner` is member-writable.
*Mine · Everyone*, the row label and "This is Mher's video." key on the ledger:
- the `owners` scope, refreshed with the list, gives the id → member map;
- ledger entries are immutable, so the map is cached for good, and a refresh needs only
  ids not yet in the cache;
- when the Worker can't answer (the repair mood), the token Mac reads each uncached id's
  ledger entry directly over S3 — a GET of `.shroom/registry/recs/<id>`, once per id,
  ever. Until an id's owner is known, its row shows in *Everyone* without a label and
  never in *Mine*, and Delete… on it shows no "This is …'s video." line.

*Rejected:* trusting `meta.owner` on token Macs (a member could file their video under
someone else's name, or under *Mine* on the owner's Mac).

### 4.11 `GET /_shroom/api/members` (owner-class)

```jsonc
200 { "shroom":1, "now":"iso",
      "owner":   { "id":"m1", "name":"Bagrat",
                   "devices":[{ "id":"d_…", "name":"MacBook Air", "joinedAt":"iso", "token":false, "self":false }] },
      "members": [{ "id":"m_…", "label":"Mher", "joinedAt":"iso" }],
      "invites": [{ "id":"i_…", "kind":"member"|"own_mac", "label":"Ani"|null,
                    "createdAt":"iso", "expiresAt":"iso", "expired":false }],
      "labels":  { "m_…":"Mher", "m_…":"Ani" } }   // every member ever: active, removed, left
```

- `members` holds active members only: removed and left rows go (SM:1564).
- `labels` holds **every** member's label, removed and left included (SM:1369-1371): the
  Library's label source on every owner-class Mac, token Macs included (H3). A revived
  member keeps their old label here (H2).
- `invites` holds **every pending invite, expired or not** — an expired member invite is
  still a row with [ New link ] (U7) — and never the code or the link. Own-Mac invites
  come back tagged `own_mac`, and **v1's Members face doesn't render them**: the spec
  draws no row for a pending own-Mac link (U4, §6.4, open question 16).

**Errors:**
- 403 `forbidden` (a member);
- 401 / 403 `removed`;
- 503.

**Serves:** `invite-born`'s rows (SM:1341-1347):
- "Mher — joined Aug 3";
- "Ani — invited just now" → "invited Aug 21 · 5 days left", from `createdAt`,
  `expiresAt` and `now`;
- expired → [ New link ];
- "You · MacBook Air — joined Sep 2 · Remove…";
- "keeps your Cloudflare token", with no Remove… (`token: true`);
- the names in `member-remove-confirm`.

### 4.12 `POST /_shroom/api/invites` (owner-class)

**Request — create:**

```jsonc
{ "id":"i_<16>", "codeHash":"<64 hex>", "kind":"member"|"own_mac",
  "label":"Ani" /* member: 1–64 after trim; own_mac: null */,
  "inviter":"Bagrat" /* the macOS short name, 1–40 */ }
```

**Request — [ New link ]:** `{"id":"<existing>","codeHash":"<new>","renew":true}`.

**Response:** 201 (200 on an idempotent replay)
`{"shroom":1,"invite":{"id","kind","label","createdAt","expiresAt","expired":false}}`.
- The server sets `expiresAt` (+7 days or +1 hour); the client can't choose it.
- **Renew** replaces `codeHash`, `createdAt` and `expiresAt` on the same id: the row keeps
  its identity, and the old code dies at once.
- **Idempotent** on `(id, codeHash)`.
- **The app builds the link itself:** `https://<host>/join/<code>`.

**Errors:**

| Answer | Meaning |
|---|---|
| 400 `bad-request` | bad label or inviter |
| 403 `forbidden` | — |
| 409 `conflict` | same id with a different hash and no `renew` |
| 409 `used` | renewing an invite that was consumed: re-fetch Members (the joined row shows) |
| 410 `cancelled` | renewing an invite another owner Mac cancelled meanwhile: re-fetch Members (the row is gone) |
| 404 `unknown` | renewing an id the roster doesn't hold (pruned, or never born): re-fetch Members |
| 409 `full` | caps |
| 401 / 403 `removed` | — |
| 503 `busy` / `space`, transport | the composer keeps its text and nothing is born |

Renew never resurrects: a cancelled or used invite stays what it is, and the app makes a
fresh invite if the owner still wants one.

**Serves:**
- `invite-born`'s composer (Invite, Return ≡ Invite) and the born link + [ Copy link ] +
  "Send Ani this link — it's good for one person, for seven days." (SM:1326-1336);
- [ New link ];
- Add another Mac's nest, "…good for one hour." (SM:1384-1385);
- N29's inviter short name (SM:1351-1353).

### 4.13 `DELETE /_shroom/api/invites/<iid>` (owner-class)

- **Response:** 204. One CAS: `state = "cancelled"`, `endedAt = now`. The record is kept
  (for 30 days, §2.3), so the page renders `join-expired`.
- **Idempotent:** already cancelled → 204. An expired pending invite is cancelled like a
  live one — that is how an expired row leaves Members (U7).
- **Errors:**
  - 409 `used`, with `member` (they joined first: Members re-fetches and shows the joined
    row, offering Remove…);
  - 404.
- **Serves:** "Cancel Ani's invite?" / "The link stops working right away." (SM:1366). The
  next peek, join or page read sees `cancelled`.
- **Member invites only, in v1's UI.** The route accepts an own-Mac id too (it is just an
  invite), but nothing in v1 offers it: the spec has no pending own-Mac row to put a
  Cancel on (U4, §6.4).

### 4.14 `DELETE /_shroom/api/members/<mid>` (owner-class)

- **Response:** 204. One CAS: the member becomes `removed` with `endedAt`, and **every
  device of that member becomes `removed`**. The label is kept forever.
- **Idempotent:** already removed or left → 204.
- **Errors:** 409 `owner` for `m1`; 404.
- **Serves:** "Remove Mher?" (SM:1365). From the next request, that member gets no mint,
  no `k` and no cookie use. Credentials already minted finish within 7 days, and the
  videos keep the label in *Everyone*.

### 4.15 `DELETE /_shroom/api/devices/<did | self>`

> **Amended by DECISIONS B38 (built):** Leave is idempotent — `DELETE devices/self` authenticates
> on the device key alone and answers 204 without writing when the Mac has already left or
> been removed; `member::Space::leave` folds 401 / 403 `removed` into done.

**`<did>`, owner-class:**
- CAS → `removed`.
- Errors:
  - 409 `self` (that's Disconnect);
  - 409 `token-holder` (the target carries the flag, §3.2);
  - 409 `not-owner-mac` (a member's device: use `members/<mid>`);
  - 404.

**`self`, any device:**
- CAS → `left`.
- If the caller is a `member` with no other active device, the member becomes `left` too.
  The row goes, and the label stays.
- `m1` never changes state.
- 401 / 403 counts as already done.

Both return 204, idempotently.

**Serves:**
- "Remove MacBook Air?" / "That Mac stops recording to your space. Its videos stay."
  (SM:1367; N33's "never one that keeps the token");
- §16 #57's deregistration on Leave, Disconnect and join-switch;
- "the row simply goes" (SM:1564).

**What the removed owner Mac sees (U8) — a spec gap.** Its next `/me` or mint reads 403
`removed`. N34's three faces are all member faces: `member-revoked` would tell the
owner's own Mac "Your access to Bagrat's space was turned off… ask Bagrat if that's a
surprise" — Bagrat's name, on Bagrat's Mac. The spec draws no state for a removed
owner-class Mac. **Proposed:** the same posture as `member-revoked` (a `config` banner
under the amber dot, Start greyed, [ Save to Movies ] while takes wait), with owner words
and the owner's exit — "This Mac was removed from your space. Recordings on this Mac are
safe — add it again from Members on your other Mac." · "Disconnect this Mac…". New copy,
open question 18. A fresh token pasted here also brings it back (enroll revives removed
`m1` devices, §4.6), but the banner doesn't advertise that: the removal was someone's
choice.

### 4.16 `POST /_shroom/api/mark` (owner-class)

- **Request:** `{"on":bool}`.
- **Response:** the same handler as the cookie route (`worker.js:275-279`):
  `.shroom/mark.json` → `200 {"on"}`.
- **Serves:** Settings → Space's mark switch on any owner-class Mac (N36). Device
  credentials never cover `.shroom/`, so Macs without the token need this route.

### 4.17 The amended page routes

> **Amended by DECISIONS B125 (built):** `/m/<id>/<file>` serves only the kinds a recording is
> made of (`.m3u8`, `.m4s`, `.mp4`, `.jpg`, `.vtt`; anything else 404), and every media answer
> carries `X-Content-Type-Options: nosniff` and `Content-Security-Policy: sandbox; default-src 'none'`.

- **`GET /<id>?k=<k>`.** Redeem as amended in §3.3: read the grant with its ETag, spend
  it by CAS (the grant plus `spent: now`, under `onlyIf: {etagMatches}`), and only the
  winner gets a cookie (H8); then delete the grant. The cookie becomes `{m,d,t}` and is
  issued only if `devices[grant.device]` and its member are active. A grant in the token
  Mac's fallback shape (§3.3, which adds `role`) redeems the same way: `role` is ignored
  and `device` is checked. A grant with no `device` (one written before this change) gets
  no cookie. A spent, expired or unknown `k`, a lost CAS, or a revoked device gets the
  same silent 302.
- **`GET /<id>`, `POST /_shroom/title/<id>`, `POST /_shroom/mark`.** `ownerOf` gains the
  live roster check (§3.5). `canEdit` keys on the ledger.
  - Title: ≤ 200 characters, B21's If-Match read-modify-write, `titled: true`.
  - Mark: owner-class only.
  - The owner page's upload line picks its waiting words from §4.18.
  - Errors: a revoked device, not-yours, or a missing or broken roster → recipient
    rendering (pages never break); POSTs → 403.
- **`GET /d/<id>`.** The fixed key, per §3.6 (done).

### 4.18 `POST /_shroom/api/health` — the page's "Waiting for your space" (§16 #44, U1)

> **Amended by DECISIONS B125 (built):** a `refused` report counts only from the recording's
> maker (its ledger entry's member) or an owner-class device — 403 `not-yours` otherwise.

**The gap.** N40's upload line has four faces (SM:946-949): "Uploading — 40%. Your link
already works." · "Waiting for connection — your link already works." · "Waiting for
your space — your link already works." (members: "Waiting for Bagrat's space — your link
already works.") · "Fully live ✓". The Worker has three today (B25). The fourth is for "a
space that serves but can't take uploads" (N40) — and then the maker's Mac can't write
the meta that would say so (A §8.1, B25's open item). So the Worker has to learn it some
other way.

**The signal: one advisory record, `.shroom/health.json`, written only by the Worker:**

```jsonc
{ "v":1, "uploads":"ok"|"blocked", "since":"iso", "why":"no-parent"|"refused", "by":"d_…"|null }
```

- **Set to `blocked`:**
  - by `/mint` itself, when it can't mint because `R2_PARENT` is missing (`no-parent`);
  - by a device's report, `POST /_shroom/api/health {"rec":"<id>","uploads":"refused"}`,
    sent when a fresh credential is refused on its first PUT (§4.8's space-side class).
    It is sent at the debounce's second answer (C7), so a blip never flips anyone's
    page.
- **Set back to `ok`:**
  - by any enroll that proved or re-placed the parent (the token Mac fixed it);
  - by a device's report `{"rec":"<id>","uploads":"ok"}`, sent once by a Mac that
    reported `refused` when its next PUT succeeds.
- **Writes only on a change.** The Worker reads the record first and writes only when
  `uploads` differs, so a stream of identical reports costs reads, never writes. It is
  an unconditional put: the record is advisory, and the last true report wins.
- **Request/response:** device key; `rec` must match B14's shape; `204`. Errors: 400,
  401/403 (ignored by the app), 503 (dropped; the next report repeats it).
- **Who may report:** any active device. A member could report falsely; §3.7 names all
  that buys (one stalled page's line). No credential, meta or roster entry depends on it.

**The page, owner viewers only.** Recipients keep N27/N40's own faces ("on its way",
"…stalled"), which promise nothing they can't know. For a viewer `canEdit` admits:

| Meta | Health | The upload line |
|---|---|---|
| `live` | — | "Fully live ✓" |
| `uploading`, progress newer than `STALL_SECONDS` | — | "Uploading — 40%. Your link already works." |
| `uploading`, stalled | `ok` or absent | "Waiting for connection — your link already works." (B25's rule today) |
| `uploading`, stalled | `blocked` | owner-class viewer: "Waiting for your space — your link already works." · member viewer: "Waiting for Bagrat's space — your link already works." |
| none, but the ledger names the cookie's member (or the viewer is owner-class) | `blocked` | **as built (B49 b, B53 c):** the owner page's pending chrome — the title field from the placeholder (a title typed meanwhile is sent once the record exists), Copy link, the player's empty frame — with the blocked words above as its upload line (N40: “opens as usual, its upload line saying so”). *Designed here:* the on-its-way layout with no title field — corrected to the spec |
| none, the ledger has an entry and the cookie may edit it | `ok` or absent | **as built (B49 b):** the same pending owner chrome while the claim is on its way, its line “Waiting for connection — your link already works.” (with no ledger entry on a clear space it is today's on-its-way face, below) |
| none | `ok` or absent | today's on-its-way face (unknown ids look like uploading ones, by design) |

- **The inviter's name** for a member viewer is `members[cookie.m].inviter`. `canEdit`
  admitted them only because `ledgerMember(id) == cookie.m`, so this is the same as
  `members[ledgerMember(id)].inviter`. The roster is already in hand from the cookie
  check: no extra read.
- **The meta-less row** is the stop N40 still opens when the space can't take the video
  (its claim never landed). For it, `/mint` writes the ledger entry **before** it looks
  at `R2_PARENT`, so a blocked first mint still records whose video it is. Ledger entries
  that never get bytes are reaped by the sweep after 30 days (§2.4).
- **Cost:** one extra read (`health.json`), only on a stalled or meta-less owner render.
  Recipients and live pages pay nothing.
- **The poll** (`/_shroom/status/<id>`) stays cookie-free and public, so it never says
  why a video waits. The owner page's script takes its waiting words from the render
  (`cfg.waiting`, `worker.js:567`). A block that starts or ends after the page loaded
  shows on the next load. *Rejected:* a cookie-checked poll (a roster read every two
  seconds per open owner page).

**The rest of §16 #44, named — still open:**
- *"Waiting for connection"'s time base* is `progress.updatedAt` aged against
  `STALL_SECONDS` (120 s). **As built (B49 c):** the uploading Mac rewrites `progress` at
  most every 5 s while the count moves and at least every 30 s while a long file is in
  flight, so the clock runs out only when nothing gets through. It can't tell offline from
  asleep or stuck: every stall that isn't a space block reads "Waiting for connection".
  Flagged only.
- *Unknown ids never reach "…stalled".* A mistyped link should land on the stalled face
  ("a mistyped link lands here too, by design", SM:1017), but it has no meta and so no
  time base; today it reads "on its way" forever. Proposed: a timer from the page's
  first load (a `sessionStorage` stamp survives the five-second refresh). Region 4's;
  not designed here.
- *Unavailable* — "the space can't serve the bytes right now" (SM:1018) — needs the R2
  binding's error shapes for storage switched off or a bucket gone, which nobody has
  recorded. Today the face shows only for a meta that is neither live nor uploading.
- *The Worker gone, mid-deploy, or over the free cap (429)* shows Cloudflare's own page:
  out of reach by construction (A §8.1).

### 4.19 `GET|HEAD /favicon.svg`, `/favicon.png`, `/favicon.ico` — the tab icon (B109)

*Standing: built. That the icon is always there, whatever the mark's switch says, is
Bagrat's to rule on (DECISIONS B109).*

- **Request:** none. No cookie is read, no key, nothing from the query. **No storage
  read.**
- **Response:** `200`. `/favicon.svg` → `image/svg+xml`; `/favicon.png` and
  `/favicon.ico` → `image/png`, the same bytes (a browser asks for `/favicon.ico`
  unprompted and reads it by its bytes). `content-length`; `cache-control: public,
  max-age=31536000, immutable`; `x-robots-tag: noindex`. HEAD answers the same headers
  with no body. Any other method → 405, as on every page path.
- **The art** is the landing's — `site/favicon.svg` and `site/favicon.png` (180×180) —
  carried inside `worker.js`: the SVG as text, the PNG as base64.
  `worker/test/icon.test.js` holds both byte-for-byte equal to the landing's files.
  Nothing is placed in the bucket for it, so wiring and the page refresh (§6.10, §6.11)
  gain no step: a space has the icon the moment it has the page.
- **Order:** answered ahead of `/<id>`, which would otherwise take `favicon.ico` for a
  video's id — an "on its way" page, or with `?k=` on the address a redeem.
- **The pages:** every face of `/<id>` and of `/join/<code>` names the icon in its head
  as the landing does — `<link rel="icon" type="image/svg+xml" href="/favicon.svg">`,
  `<link rel="icon" type="image/png" sizes="180x180" href="/favicon.png">` (Safari draws
  no SVG icon), `<link rel="apple-touch-icon" href="/favicon.png">` — after the unfurl
  tags and the title. `?embed=1` has its own head and is framed; it names none.
- **Not the mark:** the links and the answers are the same whatever `.shroom/mark.json`
  says.
- **Within iface 2:** routes only grow (§6.10); no app code calls these.

---

## 5. The Mac's disk

*Standing: recommended; extends A §3.4's whitelist.*

`~/.shroom` stays at directory mode 700. Every file is mode 600, written atomically (tmp,
fsync, rename) and carries `"v":1`. The test `Home`'s whitelist
(`app/src-tauri/src/runtime/home.rs:13`, `FILES`) gains `invite` and `join.json` —
**as built (B42):** both are in `FILES`, and `runtime/member.rs` reads and writes them. A §3.5's canary test extends to the device secret,
the join code, `k` and credential secrets: they appear in no log (quiet mode included —
the "cloud would open {url}" line at `cloud.rs:61`, H7), IPC slice, Scene or argv
(the app's own, and the `open` it spawns).

| File | Contents | Written | Read / cleared |
|---|---|---|---|
| `invite` | `{v, code, host, inviter?, own_mac?, expires?}` — or, as the installer writes it, `{v, link, arrived}` (§6.12), which the app normalizes into the first shape on read | at once when a link arrives (the installer's hand-off, the deep link, the paste door; DV #2); peek fills the rest | cleared by a 200 from join, by Stay or ✕, or by closing its `join-expired` face |
| `join.json` | `{v, host, code, device, secret, name, previous}` | **before** POST join: a fact file, not a step counter | present at boot → replay the identical join (idempotent), classified by §4.5's table, the shape rule included. Cleared on 200, on a shroom-shaped terminal answer (404 / 410 / 409 `used` / `already-here`), or on `join-space-missing` |
| `device.key` | `{v, host, device:"d_…", secret}` | after join's 200 (from `join.json`), or before enroll (token Macs; generated once and reused on retries) | runner only, for the Bearer header. **Kept while revoked** until Leave, so a re-invite can prove `previous`. Cleared by exit |
| `space.json` | `SpaceIdentity` + `device_id` (an additive serde-default field in `crates/shroom-core/src/world.rs`); `role` = `TokenHolder` \| `OwnerClass` \| `Member`; `member_id`; `owner_name` | from join's 200, or by wiring / enroll | its existence means provisioned; re-proved on entry by `/me` (token-less) or the sweep plus enroll (token Macs). Cleared by exit |
| `token.json` | unchanged | — | token Macs only. The derived pair and `R2_PARENT` are derived when needed, never written |
| `takes/<dir>/cred.json` | `{v, purpose:"upload", accessKeyId, secretAccessKey, sessionToken, expiresAt, bucket, endpoint}` | at upload start; replaced on re-mint | so a relaunch resumes within the TTL. Deleted with the take |
| `manage` credentials, `k` | — | — | memory only (`manage`: 15 minutes; `k`: one page open) |

**Owner names.** `owner_name` takes macOS's short, locale-aware form ("Bagrat"), per N29
(SM:1351-1353). **As built (B42):** `os::short_user_name()` — at enroll
(`runtime/cloud.rs`) and as an invite's `inviter` (`runtime/member.rs`); the design-time
code's `os::full_user_name()` is gone from these paths.

**What lands, act by act:**
- **Invite (inviting Mac).** Nothing. The code lives only in memory and the born state's
  link pill, matching A §3.4's "Members rows never on disk". A crash before the reply
  leaves a member invite as a row without a link: the owner sees "invited just now" and
  can cancel it. An own-Mac invite lost that way has no row (U4) and simply dies within
  the hour.
- **Member join.** `invite` → `join.json` → (200) → `device.key`, then `space.json` with
  role `Member` → delete `invite` and `join.json`.
- **Own-Mac join.** The same, with role `OwnerClass` and `member_id: "m1"`.
- **Join-switch.** The old `device.key` and `space.json` are **untouched until the new
  join's 200** (minimal's graft 7). Then: deregister on the old host (best-effort,
  3 seconds), wipe the old space's files, write the new ones. A failed join never strands
  the Mac outside its old space.
- **Token Mac wiring, `space-found`, Add a token.** `device.key` → (at `space-found`, the
  re-place of `R2_PARENT`, §3.4) → enroll → `space.json` (`TokenHolder`, `"m1"`,
  `device_id`).
- **Leave / Disconnect** (A §3.4's recipe, step 3 made concrete):
  1. Refuse while uploads drain or a recovered take waits.
  2. Offer Save to Movies.
  3. `DELETE /_shroom/api/devices/self` (best-effort, 3 seconds).
  4. Delete `token.json`, `device.key`, `space.json`, `invite`, `join.json`,
     `restore.json`, the stranded take dirs and the Library cache.
  5. Keep `prefs.json` (minus `setup_chosen`), `once.json` and `primed.json`. The login
     item stays as it was.

**Bucket-side** (not on the Mac):
- wiring seeds `.shroom/registry/roster.json` create-only (§6.11);
- the Worker writes the roster, `.shroom/keys/<hash>` (create-only), the ledger
  `.shroom/registry/recs/<id>`, the budget counters `.shroom/registry/rate/<did>` and
  `.shroom/health.json`;
- token Macs write their own ledger entries, and only on a `broken` roster move it aside
  to `.shroom/registry/roster.broken-<ts>.json` (§6.11);
- the token Mac's sweep reaps expired or spent grants, stale budget counters, and ledger
  entries with nothing behind them (§6.8).

**Bindings, the complete list** (§16 #47):
- `BUCKET` (`r2_bucket` → `shroom-space`);
- `R2_PARENT` (`secret_text` JSON).

---

## 6. Flows, step by step

### 6.1 Invite (owner-class Mac; Members, or `all-set`'s Invite someone)

1. Type a name; press Invite or Return (SM:1338-1340).
2. The runner makes `code` (12 bytes → 16 characters), `iid`, and
   `codeHash = sha256hex(code)`.
3. `POST /invites {id, codeHash, kind:"member", label, inviter: <short name>}`.
4. **201** → the pending row "Ani — invited just now", and under it the link pill
   `v.myshroom.workers.dev/join/<code>`, the filled [ Copy link ], and the said-line.
   [ Invite ] drops to outline (SM:1338-1340). Nothing is auto-copied (law 10).
5. **Transport failure** → the composer keeps its text, and a retry resends the identical
   body.
6. **Later opens** read `GET /members`. Expired → [ New link ] →
   `POST /invites {id, codeHash: new, renew: true}`, and the same row is born again.
7. The plaintext link is not re-shown after the born state; only its hash exists
   (open question 7).

### 6.2 Join — three doors, one pipeline (N29–N31)

**Doors:**
- the page's [ Open shroom ] (`shroom://join/<host>/<code>`);
- the install script's hand-off — `--join <host>/join/<code>` on the script's command
  line (SM:1463-1474), passed to the app **by file**, never on the app's argv (§6.12);
- the paste door (the welcome's "Have an invite link?", Settings → Space).

All three accept the bare, `https://` and `shroom://` forms, and normalize to
`(host, code)`.

**Arrival:**
1. Write `invite` (DV #2). **If a take is open, everything waits** (law 4; r6 note 12).
2. `GET https://<host>/_shroom/ping`.
   - Not shroom → `join-space-missing`.
   - Unanswered → the window opens in `join-waiting`'s posture — offline or "isn't
     answering", as the Mac's network says — and retries from here by itself.
     **Offline arrival (U9):** `member-welcome` needs the inviter's name, which only peek
     supplies, so the window rests in `join-waiting`'s offline face ("You're offline —
     I'll finish joining the moment you're back.") until peek answers, then becomes
     `member-welcome`. `join-waiting` is drawn only from `joining` (N31), and its line
     speaks of finishing a join nobody has clicked yet: **a DEVIATIONS candidate**, with
     the arrival faces below (open question 6).
   - Shroom → step 3, **whatever its iface**: peek is an iface-2 route, and every later
     page keeps it (routes only grow, §6.10).
3. `POST /peek` → fill `invite`. Classified by §4.4's table, the shape rule included
   (U2):
   - `pending` → step 4 — unless ping said `iface < IFACE_MIN` (below);
   - `expired` / `cancelled` / `unknown` → `join-expired`;
   - `used` → `join-used`;
   - not shroom-shaped → re-ping: not shroom → `join-space-missing`.

   **`iface < IFACE_MIN` (U3) uses the spec's own words, not `join-waiting`'s.** The spec
   has a line for exactly this — "If an app ever needs a newer page than its space has"
   (SM:1266-1270): a member invite reads "Bagrat's space needs an update — it happens
   when Bagrat updates shroom.", and an own-Mac link reads "Your space needs an update —
   open shroom on the Mac you set it up on, or add a fresh token here." + [ Add a token
   here ] (the wizard at `token`, whose `space-found` makes this Mac a token Mac, which
   then refreshes the page itself — §6.10). Peek runs first, so the inviter's name is
   there. The face takes `join-waiting`'s posture — the spinner rests, no Join, nothing
   consumed — and resumes by itself to `member-welcome` when a later ping reports enough.
   The words are verbatim; their place (the invite window, where the spec puts them on
   `config`) is **a DEVIATIONS candidate** (open question 15). *Rejected:*
   `join-waiting`'s "isn't answering" line (it says the space is silent when it is
   answering, and hides the one thing that will fix it).

   Arrival-time failure faces (`join-expired`, `join-used`, `join-space-missing` before
   anyone clicks Join) are a DEVIATIONS candidate too (open question 6). Spec-literal,
   these faces are reached only from `joining`.
4. Compare with this Mac:
   - **No space** → `member-welcome`, or its own-Mac variant from `kind`.
   - **Same host** → `GET /me` with the stored device key.
     - 200 → `join-already-here` (SM:1542). The link stays unused. **Its wording follows
       this Mac's role, not the invite's kind (U5):** an owner-class Mac (token Mac or
       not) reads the own-space line, "This Mac is already in your space — open the
       link on the Mac you want to add." — whichever kind of link it opened, since
       "Bagrat's space" on Bagrat's own Mac is the wrong voice. A member's Mac reads
       "This Mac is already in Bagrat's space — nothing to do. The invite is still
       unused, so it can go to someone else." — an own-Mac link that strayed to a
       member's Mac included.
     - 401 / 403 → this identity is dead → `member-welcome`, with `previous` set
       (SM:1588).
     - 503, or unanswered → `join-waiting`'s posture until `/me` answers; the link stays
       unused either way.
     - not shroom-shaped → re-ping (`join-space-missing` if not shroom).
   - **Another host, or a half-set-up space** (`token.json` without `space.json`) →
     `join-switch-confirm`.
     - Join is greyed while uploads drain or a recovered take waits (SM:1555-1559).
     - **The only-token-holder line (U9)** shows when this Mac is a `TokenHolder` and the
       old space's `/me` reports `tokenHolders <= 1` with this Mac counted — **and also
       when the old space can't be asked** (offline, or `/me` unanswered): the line is
       true for any token Mac whose space exists, so the default is to say it (§4.7).
       It is dropped only when the old space answers as not shroom ("everything will be
       there" would be false).
     - The half-set-up line per SM:1561-1563.

**Join** (or [ Join Bagrat's space ]):
1. Write `join.json`, then `POST /join`. The row reads "Joining — a few seconds."
2. **200** → `device.key` and `space.json`. On a switch, the old space is deregistered and
   wiped now. Delete `invite` and `join.json`. The row ticks with the fact →
   `done · member` with its nest → Region 3's chapter → `all-set · member`: two doors, or
   three for own-Mac.
3. **Other answers** map per §4.5's table, the shape rule included: a non-shroom answer
   re-pings and can land on `join-space-missing` (SM:1541) or the needs-update face
   (U2, U3). `join-waiting` replays the identical body on reconnect, and a crash mid-call
   replays from `join.json` at boot through the same table.
4. **No credentials are minted at join.** They mint per recording (SM:1514-1518).

**Later** stores the invite. A tray click reruns peek: an invite that has died shows
`join-expired` once, then the plain `welcome` (SM:1497-1499).

### 6.3 Re-invited after removal

1. The revoked Mac kept `device.key` (§5). Its `/me` reads 403 `removed`.
2. The new link arrives, and step 4 of the arrival finds the identity dead →
   `member-welcome`.
3. Join sends `previous = {id, secret}`. The Worker revives **the same member id and
   keeps its old label** (H2); the new invite's label goes into the member's
   `reinvites` history only. The owner's pending row for this invite becomes the old
   member's row ("Mher — joined Oct 2"), whatever name the invite was typed under.
4. **Waiting takes drain**, because their ledger entries name that member (SM:1588-1589).
   *Mine* continues.
5. If more than 180 days have passed, the dead device was pruned. The Mac joins as a new
   member under the new invite's label, and its old takes read `not-yours`, kept locally
   and offered Save to Movies (open question 8).

### 6.4 Add another Mac (N33)

1. *Add another Mac* on the You row → `POST /invites {kind:"own_mac", label:null}` (one
   hour) → the nest: the link, [ Copy link ], "Open this link on your other Mac — it's good
   for one hour."
2. On the other Mac, the same pipeline runs with the own-Mac words from peek's `kind`.
3. Join sends `device.name` (the sharing name). The Worker adds the device under `m1`,
   with role `owner`.
4. The new Mac's `all-set` keeps three doors, and its Settings shows Members.
5. On the first Mac it appears as the sub-row "You · MacBook Air — joined Sep 2 ·
   Remove…".

**While the link is pending (U4).** The spec draws the nest and the joined sub-row, and
nothing in between: there is no row for a pending own-Mac link, so nothing to put a
Cancel on. This doc follows the spec — **no Cancel for own-Mac links in v1** — and
§3.7's mitigation for a leaked one is the spec's own: one hour, one use, and an
unexpected Mac under You that Remove… ends (SM:1389-1392). `GET /members` returns pending
own-Mac invites tagged `own_mac`, and the face skips them. If Bagrat wants the row,
open question 16 proposes it: while the link lives, the nest stays under You as
"A new Mac — link good for 52 more minutes · Cancel…", confirming with
"**Cancel this link?**" / "The link stops working right away." [ Keep it ] ···
[ Cancel link ] — new copy, a fourth face of `member-remove-confirm`, routed to
`DELETE /invites/<iid>` (which already accepts it).

### 6.5 Remove a member, cancel an invite, remove another Mac

All three come from `member-remove-confirm` (no Return default, law 9).
- **A member** → `DELETE /members/<mid>`. Their next mint, `k` or cookie use fails. Their
  in-flight uploads finish on credentials already minted (≤ 7 days, SM:1365).
- **A pending invite** (a member invite — own-Mac links have no row, §6.4) →
  `DELETE /invites/<iid>`. The next peek, join or page read sees `cancelled`. A 409
  `used` means they won the race: re-fetch and show the joined row.
- **Your other Mac** → `DELETE /devices/<did>`. Refused for self and for a token-holder;
  the UI never offers either (SM:1394-1399).

### 6.6 Leave, Disconnect, switch

This is §5's exit recipe.
- **The owner sees** the member row simply go, while the videos keep their label
  (SM:1564-1565). For an `m1` device, only that Mac's sub-row goes.
- **Offline:** the wipe proceeds, and the stale row stays until the owner removes it. A
  token Mac's stale row stays un-removable until its flag lapses (open question 9).
- **A token Mac's Disconnect** removes its own device record. Its token stays in
  Cloudflare until deleted there (SM:1396-1399).

### 6.7 Member recordings and uploads (B32 mints)

1. **Capture start** (DV #4) → `POST /mint {rec, purpose:"upload"}` → the Worker writes
   the ledger and mints → `cred.json`.
2. **A token Mac** mints locally from the derived pair, and writes the ledger
   create-only over S3 before its first segment.
3. **A §9.1 runs as today:**
   - claim `meta/<id>.json` with `If-None-Match: *` at Stop or Get link, `owner` =
     `response.owner` (the ledger's member, C4);
   - segments → playlist last (the go-live) → master → meta `live`;
   - every rewrite under B21's If-Match, stopping on `deleted`.
4. **Expiry** → re-mint the same `rec` (allowed: the ledger matches).
5. **A fresh credential refused on its first PUT** (space-side, §4.8) is reported once
   the debounce confirms it — `POST /health {rec, uploads:"refused"}` — and `ok` is
   reported once the next PUT lands (§4.18). Every Mac reports, a token Mac whose own
   token died included (its device key still works). That is what lets the owner page
   say "Waiting for your space".
6. **Refusals classify member life on every entry**, and each face clears by itself
   (law 1, N34):

| Cause | Face | Start |
|---|---|---|
| mint or `/me` 401, or 403 `removed`, on a member's Mac | `member-revoked` | greyed |
| the same on an owner-class Mac without the token | the removed-Mac face proposed in §4.15 (U8; a spec gap, open question 18) | greyed |
| mint or `/me` 503 `space`, or a fresh credential refused on its first PUT — **twice, at least 30 s apart** (C7) | `space-needs-owner` | live |
| ping not shroom, or NXDOMAIN while online — **twice, at least 30 s apart** | `space-gone` | greyed |
| ping `iface < IFACE_MIN` | the needs-update sibling line (SM:1266-1270) | live |

The debounce (§4.8) is applied to both space-side rows, `space-gone` included: a DNS
blip or one R2 hiccup must not grey Start. A single such answer is a silent backoff.

On an owner-class Mac without the token, the space-side rows read Region 2's own words,
not N34's member faces: `space-degraded`'s token-less banner, "Your space needs a fix —
open shroom on the Mac you set it up on, or reconnect here with a fresh token."
[ Reconnect here ] (SM:659-661). Only the removed row lacks a spec face (U8).

**Owner-class Macs without the token** use the same door. The door is chosen by token
presence (#58).

### 6.8 Member Library and delete scope

**Members:**
- **Open** on the local cache.
- **Refresh:** `POST /library {scope:"mine", known, knownThrough, after}` pages until
  `next` is null, each call carrying the next slice of the cache (§4.10); deltas slide
  in.
- **Rename / Delete…** (their own videos only):
  1. `POST /mint {rec, purpose:"manage"}` (15 minutes).
  2. The unchanged `shroom-upload::library::{rename, delete}`: B21's read-modify-write,
     and tombstone-then-delete.
- **Offline:** Copy link stays live; rename and delete grey out (A §13).

**Owner-class Macs without the token:**
- the same calls with `scope:"everyone"`, with labels from `GET /members`'s `labels`;
- the same `manage` mints for any id.

**Token Macs (H3):** their own S3 list for titles and states, the `owners` scope (or a
direct ledger GET per uncached id, in the repair mood) for ownership, and `/members`'
`labels` for names — never `meta.owner` (§4.10).

**Delete under a `manage` credential** needs a prefix list and DeleteObjects under a JWT
credential (L2 P4). **Fallback:** delete the fixed B13 layout, using the names in the
playlist (`master.m3u8`, `v0/init.mp4`, `v0/stream.m3u8` plus the segments it lists,
`video.mp4`, `poster.jpg`, `teaser.mp4`), then let the token Mac's sweep reap the rest.

**The sweep (token Macs only — it lists `rec/`, which no device credential can).** Its
byte-level core exists: `shroom-upload::library::sweep` (commit `2c6f8de`,
`crates/shroom-upload/src/library.rs:161`) reaps bytes under a tombstone, and record-less
bytes for ids the caller passes as stale (`orphans_ok`). This doc adds what it is told and
what else it reaps:
- `orphans_ok(id)` = the id has no meta and its ledger entry is older than the upload
  TTL plus a day (8 days): past that, no credential can still be writing it. Bytes with
  neither a meta nor a ledger entry shouldn't exist once Region 6 lands (every upload
  writes its ledger entry before its first byte); the sweep leaves them and logs them;
- ledger entries with neither a meta nor any `rec/` bytes, 30 days after `at` (§2.4);
- `.shroom/keys/<hash>` grants past `exp`, spent or not (§3.3, H5);
- `.shroom/registry/rate/<did>` counters whose hour is over.

**Still open from §5.5 / §16 #49** (named, not designed here):
- *who runs the sweep and when* — a token Mac, but on what trigger (each entry, daily,
  after a delete) is unset;
- *multi-select delete* — N35's ⌘/⇧-click "Delete… once" means one `manage` mint per id,
  sequentially; there is no batch mint, and a partial failure's row states are unset;
- *Discard of a never-shared take, offline or after its credential expired* — online, a
  member re-mints `upload` for the same id (the ledger matches; `manage` needs a meta,
  which a never-shared take lacks) and runs `library::discard`. Offline, the local take
  goes now and its bucket bytes become record-less; the sweep's `orphans_ok` rule above
  reaps them after 8 days. Whether that delay is acceptable is unasked;
- *whether a tombstone keeps `owner`* — `library::delete` keeps it today
  (`library.rs:122-125`), which keeps "This is Mher's video." answerable from the ledger
  either way.

### 6.9 Member owner-mode cookie, and `k` off argv

1. **Stop, Get link, Open page or the deferred open:**
   - make `k` (32 bytes);
   - `POST /key {kHash}`;
   - open `https://<host>/<id>?k=<k>`.
2. The Worker redeems it → cookie `{m,d,t}` → 302 to the clean URL.
3. A member's own videos get the editable title, with no mark switch (N40).
4. Removing the device or member ends the cookie on its next use.
5. **Law 13: the open goes through `NSWorkspace openURL:`** via `objc2-app-kit` (already a
   dependency with the `NSWorkspace` feature, `app/src-tauri/Cargo.toml:18`; add `NSURL`
   to `objc2-foundation`). Never `Command::new("open").arg(url)`.
   - **Where.** When this was written, `OpenOwnerPage` opened the clean URL that way
     (`app/src-tauri/src/runtime/cloud.rs:63`). The moment `?k=` is wired in, `k` would
     sit in a process's argv, visible in `ps`, which law 13 doesn't allow. **As built
     (`6ff5d0e`, B42; B36's open note closed):** `?k=` opens through `NSWorkspace openURL:`
     (`runtime/cloud.rs`, `open_privately`), and a quiet run logs only “cloud would open
     the owner page for <short id>”.
   - **And the log line two lines above it (H7).** In quiet mode the same handler logs
     `"cloud would open {url}"` (`cloud.rs:61`). With `?k=` wired in, that would write a
     live `k` to `run.log`. It must log the clean URL — or just "cloud would open the
     owner page for <id>" — and never the keyed one. The canary test (§5) runs in quiet
     mode and asserts no `k` in the log.
   - **Scope.** This lands with the `?k=` wiring for every owner-page open. The other
     `open` calls carry no secret and may stay: `os::open_detached` (`os.rs:67`, called
     from `os.rs:60` for public pages and from `mod.rs:474` for System Settings panes),
     the Screen Recording pane (`os.rs:102`), the Finder reveal (`takes.rs:182`) and the
     dev harness's Finder activation (`app/src-tauri/src/harness.rs:347`, dev builds
     only).
   - Logged in DECISIONS when built (must-fix, not an open question).

### 6.10 Page version (`iface`)

- **`worker.js` carries `const IFACE = 2, VERSION = "<semver>"`**, and ping reports both.
  Routes and fields only grow within an iface. A breaking change takes a new path and an
  iface bump.
- **Each app release compiles `IFACE_MIN`**, the oldest iface whose routes it calls: 2 for
  the first Region 6 release.
- **Token Mac**, on update apply, repair or `space-found`: deploy iff the bundled
  `VERSION` is greater than `ping.version` (semver), **or** the self-check or enroll found
  bindings missing or stale. Never an older version over a newer one (SM:1754-1756).
  `space-found` is in the list so that [ Add a token here ] — offered by the
  needs-update line below — actually brings the page up to date.
- **Every other Mac** only reads:
  - `iface < IFACE_MIN` → members read "Bagrat's space needs an update — it happens when
    Bagrat updates shroom.";
  - owner-class Macs without the token read "Your space needs an update — open shroom on
    the Mac you set it up on, or add a fresh token here." + [ Add a token here ]
    (SM:1266-1270);
  - **a Mac that is joining** reads the same line by the invite's kind — the member line
    for a member invite, the owner-class line for an own-Mac link — in the invite window
    (U3, §6.2; a DEVIATIONS candidate for the placement, open question 15);
  - a Worker ahead of the app is always fine.
- **As built:** `IFACE` 2 and `VERSION` in `worker.js`, `IFACE_MIN` 2 in
  `shroom-cloud::member`. The deploy rule compares `VERSION` (never `iface`, B50's rejected
  alternative): wiring reads the page first and keeps a page at this version or newer
  (re-placing `R2_PARENT` alone, B50 a); the token Mac checks its page once a launch and
  puts its own up only over an older `VERSION` (`wire::refresh_page`, B50 c) — a check
  that couldn't run (no answer, not shroom) is a failure retried, never “checked” (B53 a).
  `VERSION` moves with every page change (`page-version.test.js`, B61). The joining Mac's
  placement is built as recommended (B41 n) — DEVIATIONS #30.

### 6.11 Wiring and update additions (§16 #46/#47; must-fix 7)

**Wiring order** (`wire.rs`):
1. bucket;
2. player assets;
3. `cookie.key` (create-only);
4. **`roster.json` seeded create-only** `{v:1, createdAt, members:{}, devices:{},
   invites:{}}`;
5. the script PUT **with `R2_PARENT`**;
6. the route;
7. the self-check;
8. **`POST /enroll`**: a `stale-parent` here means the PUT didn't take; the step
   re-places the secret through the secrets endpoint (§3.4) and enrolls again, and a
   second `stale-parent` fails the step, which retries;
9. **the manifest, last.** A cut-short wiring never reads as `space-found`.

**`space-found`** (a fresh paste finds the space): it deploys only if the bundled page is
newer (§6.10), so it **re-places `R2_PARENT` explicitly** either way (§3.4's second
moment), then enrolls. Without the re-place, the Worker would keep minting from the
previous token's pair — dead, if that token is why the owner pasted a fresh one.

**As built (B37, B42, B50):** wiring runs bucket → player → `cookie.key` → `roster.json`
(create-only) → the script PUT with `R2_PARENT` (or, over a page already at this version or
newer, `R2_PARENT` re-placed alone) → the route → a wait of up to 90 s for the page to
answer → the self-check → the manifest, last. Enroll runs after `Wired`, non-fatal (a
token Mac also enrolls when it meets a 401 or has no key); its `stale-parent` re-place is
`enroll_here`'s.

**The token Mac's update path** runs the same deploy step, so `roster.json` is seeded
(create-only, a no-op if present) **before** an iface-2 script goes up. **As built:**
`wire::refresh_page`, once a launch (B50 c).
- Spaces wired before Region 6 (Bagrat's own and the fenced test spaces) get their roster
  on the first update. Until then, Region 6 routes answer 503 `space`/`unwired`.
- That is unreachable for members, who can only arrive through an invite, which needs the
  roster. It never shows on Bagrat's own Mac, because the update seeds before it deploys.

**Every entry on a token Mac** calls enroll, after the sweep has found its token live:
- it stamps `proofAt` at most once a day (C5, §4.6) — most entries write nothing;
- **a `stale-parent` or `no-parent` answer re-places the secret and enrolls once more**
  (H1). Two token Macs holding different live tokens therefore each re-place on entry;
  §3.4 explains why that is harmless and what it costs;
- a dead token never re-places: it is the repair mood's `token` row.

**A `broken` roster — the recovery (C8).** The Worker answers 503 `space`/`broken` when
`roster.json` is present but won't parse. Nothing the Worker writes can cause that (it
writes `JSON.stringify` output, and R2 puts are atomic); the realistic cause is a hand
edit in the dashboard. Every Region 6 route fails closed until it's fixed: members read
`space-needs-owner` (debounced), cookies read as recipients, joins wait. Recovery is the
token Mac's, in the repair mood (the "Wiring it up" row, SM:651):
1. read `roster.json` over S3 and confirm it doesn't parse (a transient read error is not
   `broken` — retry);
2. copy it aside, create-only, to `.shroom/registry/roster.broken-<ts>.json` — kept for
   hand repair, never read by the Worker;
3. **if Bagrat confirms a reseed**, write a fresh roster (an unconditional PUT over S3,
   the one time anything but `registry::seed` writes it) and enroll. `m1` and this Mac
   come back at once. **Every member, every other owner Mac and every pending invite is
   gone**: their devices read `unauthorized` → `member-revoked`, and each needs a fresh
   invite. Their recordings keep their ledger entries, but a re-invited member gets a
   new member id (no `previous` can match a device the roster no longer holds), so their
   old videos stay with the old id: unlabelled in *Everyone*, and missing from their own
   *Mine*. The labels survive only in the set-aside copy;
4. otherwise the space stays in the repair mood until the copy is fixed by hand and put
   back.

The spec has no line for step 3's consequence, so the reseed is **not automatic**: it
waits on open question 19. *Rejected:* a Worker-kept `roster.prev.json` rewritten with
every change (a second Class A write per registry change, to guard against a hand-edit
accident).

### 6.12 The installer's hand-off (H9; amends B29)

**The problem.** B29's script ends with `open "$APP" --args --join "$join"`
(`installer/install.sh:130`). That puts a live join code — for an own-Mac link,
owner-class for an hour — in the app's own argv for as long as the app runs: days, for a
menu-bar app, readable by any process through `ps`. Law 13's exception covers the
install command once (SM:153-155); the app's argv is a second, long-lived copy.

**Chosen: hand it over by file.** The script writes the link where DV #2 already keeps
an arriving invite, then opens the app with a flag that carries no secret:

```sh
# 4. Open it. A join link goes to the app by file (mode 600), never on its command line.
if [ -n "$join" ]; then
  case "$join" in
    *[!A-Za-z0-9._:/-]*|"") join="" ;;   # not a link's shape: open shroom plainly
  esac
fi
if [ -n "$join" ]; then
  ( umask 077; mkdir -p "$HOME/.shroom" &&
    printf '{"v":1,"link":"%s","arrived":"%s"}\n' "$join" "$(date -u +%Y-%m-%dT%H:%M:%SZ)" \
      > "$HOME/.shroom/invite.tmp" && mv -f "$HOME/.shroom/invite.tmp" "$HOME/.shroom/invite" )
  open "$APP" --args --invite </dev/null
else
  open "$APP" </dev/null
fi
```

- **The shape check** keeps the JSON trivially safe (no quote or backslash can pass it)
  and lets nothing but a link's characters into `~/.shroom`. The app still classifies
  the link itself (N29) and normalizes the file into its own `invite` shape (§5).
- **`--invite`** marks the launch as the invite's arrival, so it opens the invite even in
  the otherwise silent updating lane (N41) — the same provenance `--join` gave, without
  the secret. A launch without it treats an `invite` file as remembered (Later), never
  as new.
- **Mid-take:** unchanged — the script refuses before it writes anything (B29), so a take
  never meets a fresh file, and the app's own arrival path waits out a take anyway
  (law 4).
- **The script's readable header** (ratified, SM:233-291) doesn't change; its body grows
  by a few lines, and its four steps stay four.
- **What changes:** `installer/install.sh` (step 4), `installer/test/install.test.mjs`
  (the invite hand-off test asserts the file, its mode 600, its shape, the `--invite`
  flag, and that no argv the script spawns after the swap carries the link), the app's
  launch handling (`--invite` replaces `--join <link>`), and B29's DECISIONS row,
  amended when built.
- *Rejected:* the deep link, `open "shroom://join/<host>/<code>"`. The code would ride
  `open`'s argv only briefly (law-wise no worse than the script's own), but a freshly
  placed app may not have its URL scheme registered yet (r6 scaffold flag 3:
  registration happens at first launch), so on a fresh install the hand-off could fail
  silently. The file always lands.
- *Rejected:* keeping `--args --join <link>` (the problem above).

---

## 7. Tests to write

### 7.1 L1 — Worker unit tests (`worker/test/`, vitest, `fake-bucket.js`)

**The fake bucket first gains:**
- `onlyIf.etagMatches`, returning `null` on mismatch;
- create-only;
- `list({prefix, startAfter, limit, include:["customMetadata"]})`, with an injectable
  short page (`truncated: true` with fewer than `limit` objects), so no code can lean on
  the count (P3);
- an injectable "throw on precondition" mode, for both of the CAS contract's branches,
  and an injectable per-key write-rate refusal (C5).

**The P0 tests — partly done (`ff84bb7`):**
1. **Key confinement — done for the read routes.** `worker/test/worker.test.js` drives
   `/<id>`, `?embed=1`, `/d/`, `/m/`, `/_shroom/status/` and `/oembed` with hostile meta
   fields and asserts the keys read stay within the allowlist. **To do:** extend it to
   every route this doc adds, with hostile path segments and bodies, against §3.6's
   per-route additions.
2. **Done:** `/d/<id>` streams `rec/<id>/video.mp4`, and a record naming
   `.shroom/cookie.key` gets a 404 with no secret in the body.
3. **To do:** no response body or header carries the bytes of `R2_PARENT` or
   `cookie.key`, across every route and error path.

**Registry:**
4. `mutate`: a stale ETag retries from a fresh read; 5 losses → 503 `busy`; a thrown
   precondition and a write-rate refusal are each treated like `null`; a missing roster
   → 503 `unwired`, an unparseable one → 503 `broken`, and the roster is never created.
5. Pruning and caps (U7): pending member invites are never pruned, expired or not;
   used invites survive while their device does; cancelled ones go after 30 days, expired
   own-Mac ones after 30 days; members never; devices after 180 days. Only unexpired
   pending invites count toward the cap of 20. Each cap → 409 `full`. NFC and
   control-character stripping on `label`, `name` and `inviter`.

**Join machine:**
6. **Single use:** two joins with different devices, interleaved at the CAS → exactly one
   200, the other 409 `used`.
7. **Cancel vs consume race:** both orders give exactly one winner, and the loser's class
   is right (410 `cancelled` or 409 `used`).
8. **Idempotent replay:** same code, did and verifier → the identical 200, no second
   member.
9. `previous` (H2): revives a removed member **with its old label unchanged** and the
   invite's label in `reinvites`; an active device → 409 `already-here` with the invite
   still pending; never revives `m1`; a bad secret is ignored.
10. Peek and the join page never write. Expired is derived from the clock. Lifetimes are
    7 days and 1 hour, set by the server.
11. Every join-page face renders its spec line verbatim (extending `copy.test.js`), with
    `noindex`, `no-referrer` and `frame-ancestors 'none'`; every face carries the mark
    whatever `mark.json` says, and the route never reads `mark.json` (U6). A missing
    roster renders the transient line, never "ask for a fresh one".
12. **Escaping (H10):** an invite whose `inviter` is `<script>alert(1)</script>` and whose
    `label` is `"><img src=x onerror=alert(1)>` renders, on every face and in
    `<title>`, with no `<script`, `<img` or unescaped quote reaching the HTML — both
    as a recipient and in the owner view.

**Auth:**
13. Bearer: unknown → 401; wrong secret → 401; removed device or member → 403 with `why`;
    the role comes from the roster, never the body.
14. `Origin` present on any `/_shroom/api/*` → 403.
15. Parent proof: the right key, a skew > 300 s → `clock`, another key → `stale-parent`, a
    JWT-shaped message → rejected (domain separation), a body swap → rejected. Enroll
    only ever writes under `m1`, revives a removed `m1` device (U8), and writes nothing
    when `proofAt` is under 24 hours old and nothing else changed (C5).

**Mint:**
16. **The golden vector (C1):** the JS JWT is byte-equal to `mint.rs`'s for fixed inputs,
    with the claims in alphabetical order. The Rust side asserts the same vector.
17. First upload mint creates the ledger (before the parent is read: a `no-parent` mint
    still leaves the entry); another member's later mint → 403 `not-yours`; owner-class
    `manage` on any id; a member's `manage` on others' → 403; `deleted` → 410; legacy
    ids (no ledger, meta present) → `m1` only; the response's `owner` is the ledger's
    member (C4).
18. **Exactly two paths (H6):** every minted JWT's `paths` is exactly
    `{"objectPaths":["meta/<rec>.json"],"prefixPaths":["rec/<rec>/"]}` for the request's
    own `rec` — nothing under `.shroom/`, nothing else. `rec` values of 15 or 17
    characters, or holding `/`, `.`, `%2F` or `..`, get 400 before any key or claim is
    built.
19. **The upload budget (H4):** the 21st first mint from one device in an hour → 429
    `too-many`; re-mints of an existing id don't count; another device is unaffected.

**Cookie and `k`:**
20. The cookie `{m,d,t}` round-trips. A removed device's cookie renders a recipient and
    is refused on title and mark. A cookie without `d` is a recipient. A member's title
    applies only where the ledger names them. The mark is owner-class only. With the
    roster `broken`, every cookie renders a recipient (C2).
21. `key` (H5): a `kHash` that isn't exactly 64 lower-case hex characters → 400 (including
    `../`, upper case, 63 or 65 characters); the grant is created create-only; a replay
    from the same device is a 200 and leaves the first grant; a different existing
    grant → 409. Redeem refuses a grant whose device has since been removed.
22. **Concurrent redeem (H8):** two redeems of one `k`, interleaved between the read and
    the CAS → exactly one `set-cookie`; the other gets the plain 302.

**Members and Library:**
23. Members: removed and left members are absent from `members` but present in
    `labels` (H3); a revived member's label is the old one (H2); pending invites include
    expired ones (U7) and own-Mac ones tagged `own_mac` (U4); `token` true only while
    `proofAt` is under 30 days, whatever `tokenKey` says (H1).
24. Library: members are forced to `mine`, and to nothing else (`owners` → 403); ETag
    deltas; ≤ 40 bodies, then `next`; tombstones returned as `deleted`; a short
    `truncated` page from either list is followed, not taken as the end, and no page
    reaches past `knownThrough` (D); the `owners` scope returns the whole map across
    pages, uncapped.
25. Health (U1): a `no-parent` mint marks the space `blocked`; a device's `refused`
    report does too, and its `ok` report clears it; identical reports write nothing; a
    stalled owner page reads "Waiting for your space — …" for an owner-class cookie and
    "Waiting for Bagrat's space — …" for a member's own video; a recipient's page never
    changes with health; `/_shroom/status/<id>` never carries it.
26. `iface` and `version` in ping; old and new app pairs: an app with `IFACE_MIN 2`
    against fakes reporting 1 and 3.

**Rust side** (`shroom-cloud`, `shroom-core`, the app):
- the golden vector;
- `registry::seed` is create-only; `registry::set_aside` copies a broken roster
  create-only and never deletes it;
- `join.json` replay at boot, through the shape rule: a non-shroom answer re-pings, and
  a non-shroom ping lands on `join-space-missing` (U2);
- arrival with `iface < IFACE_MIN` → the needs-update face with the spec's words, not
  `join-waiting`'s (U3);
- `join-already-here`'s words by this Mac's role: owner-class + a member invite → the
  own-space line; member + an own-Mac link → the named line (U5);
- the only-token-holder line shows when `/me` is unreachable, and not when the old
  space answers as not shroom (U9);
- the switch order (the old files are untouched until 200);
- the exit recipe's file list against the `Home` whitelist (which now holds `invite`
  and `join.json`);
- the canary (no device secret, code or `k` in logs, IPC or argv), run in quiet mode so
  `cloud.rs:61`'s log line is covered (H7);
- the owner-page open uses `NSWorkspace`, never `open` argv;
- the debounce: one space-class answer shows nothing; two, 30 s apart, show
  `space-needs-owner`; a success between them resets (C7);
- token Macs: `Mine`, labels and "This is Mher's video." come from the ledger map, never
  from a `meta.owner` the fixture forged (H3);
- enroll's `stale-parent` with a live token re-places the secret once and enrolls again;
  with a dead token it never calls the secrets endpoint (H1);
- every refusal class → its N34 / N31 face, in `derive` tests, and an owner-class Mac's
  403 `removed` → the removed-Mac face, never `member-revoked`'s words (U8).

**Installer** (`installer/test/install.test.mjs`, H9):
- `--join <link>` writes `~/.shroom/invite` with mode 600 and `{v, link, arrived}`, then
  opens the app with `--invite` and no link on any argv after the swap;
- a value that isn't a link's shape writes nothing and opens the app plainly;
- mid-take still refuses before anything is written.

### 7.2 L2 — live, fenced probes (`shroom-cloud/tests/*_live.rs`, the test space)

**These gate Region 6 code** (must-fix 4). Each has a named fallback — except P1, which
is a **hard gate** (C4): no Region 6 code is written until it passes, and if it fails,
the registry's store goes back to Bagrat with §2.5's Durable Object delta.

| # | Probe | Pass | Fallback |
|---|---|---|---|
| P1 | **Hard gate.** (a) Two concurrent `put(onlyIf:{etagMatches})` on one key through the binding. (b) Writes to one key faster than one a second (C5): what R2 answers, and whether a refused write is distinguishable from a lost CAS | (a) exactly one wins; the loser sees `null` (or a throw, recorded). (b) the answer is recorded, and `mutate`'s retry absorbs it | none in-frame: the store becomes Bagrat's call again (§2.5) |
| P2 | Create-only through the binding (`onlyIf` with `If-None-Match: *` headers, or `etagDoesNotMatch:"*"`) for the ledger, the budget counters and the `k` grants | a second create refuses, and the first survives | the Worker signs one SigV4 S3 `PUT If-None-Match: *` with `R2_PARENT` (the probed B15 path; about 60 lines of WebCrypto) |
| P3 | `list({include:["customMetadata"]})` returns the ledger's `m` (including for entries a token Mac wrote over S3 with `x-amz-meta-m`); how many objects a page holds with `include`, and that `truncated` and `cursor` are set when it stops short (D) | the map comes back page by page, and a short page says `truncated` | a GET per ledger entry, bounded per call like the meta bodies |
| P4 | ListObjectsV2 under prefix and DeleteObjects under a `manage` JWT credential | both succeed inside `rec/<id>/`; 403 outside | delete the B13 layout named by the playlist (§6.8) |
| P5 | `R2_PARENT` `secret_text`: placed in script-upload metadata, readable as `env`, re-placed on a later PUT, survives a PUT that re-sends it; **and re-placed alone through the script-secrets endpoint under the two-permission token, with no redeploy (H1)** | — | `keep_bindings` on the upload; for the re-place, a full script PUT |
| P6 | A Worker-minted credential PUTs inside `rec/<id>/` and is refused at `.shroom/`; **a credential with `exp = iat + 604800` is accepted (B32 probed only 600 s), and one past R2's ceiling, if it has one, is refused (D)** | — | for the signer, none needed (the golden vector pins it); for the TTL, mint R2's ceiling and re-mint on expiry (§3.4) |
| P7 | The workers.dev answer for a label with no Worker, and NXDOMAIN for a removed subdomain | distinguishable from offline | treat both as "not shroom" only while the Mac's network path is satisfied |
| P8 | The free plan's per-invocation ceiling on binding operations | the Library page bound is set from it (40 to start) | a smaller page |
| P9 | End to end, in the fenced space: wire → enroll → invite → peek → join (member and own-Mac) → mint → upload → cookie title → remove → mint 403 → re-invite with `previous` (old label kept) → the take drains → a second token pasted on another Mac (`space-found` re-places the parent; the first Mac's next enroll re-places it back) → leave | — | — |

### 7.3 Build order

1. ~~The P0 fix and tests (§3.6), on its own commit~~ — **done** (`ff84bb7`); the
   allowlist test grows with each route below.
2. P1 (the hard gate), then P2–P8.
3. Seed, `R2_PARENT` and its three placements, enroll, ping iface 2 (wiring, the update
   path, `space-found`).
4. Join machine routes and the app's join pipeline; the installer's file hand-off
   (§6.12).
5. Mint (with the budget), key and the cookie amendment (plus the `NSWorkspace` open
   and the quiet-mode log fix), health.
6. Members routes and the Members face.
7. Library route (with `owners`), and the token Mac's ledger map.
8. P9.

Spec tests for each region are written and committed before its code (CLAUDE.md).

---

## 8. Rejected alternatives

| Rejected | For | Why not |
|---|---|---|
| A Durable Object registry | §16 #3 | Sound, but loses every member if the script is deleted. Needs migrations on every deploy and unprobed powers. Kept as §2.5's delta |
| KV registry | §16 #3 | Eventual consistency of 60 s or more breaks "stops working right away" and single use |
| A multi-object registry (per-member and per-device keys, commit-point ordering) | security design | Remove becomes several CASes. One document gives one atomic write per transition |
| Ed25519 / ECDSA request signatures, a ±120 s window, an R2 nonce object per call, list-based rate counters, R2 lifecycle rules | security design | Over TLS to one verifier, a hashed bearer buys nearly the same. Adds a Class A write per call, Ed25519-in-Workers and lifecycle-under-token unprobed, and clock handling on every call |
| Device key in the Secure Enclave or a Keychain item | security design | The Enclave is P-256 only and needs a paid team entitlement (OSS posture). The Keychain brings ACL prompts across re-signed updates (the POC's TCC saga) |
| The Worker generates the device secret and returns it in join's response | minimal design | A lost response would strand the Mac. Mac-made secrets make every retry idempotent |
| Deriving recording ids from the device key (B14 amendment) | security design | The ledger alone closes everything but the offline window, at this scale. It would also change a probed decision |
| A bucket-wide 1-hour `space` credential for token-less owner Macs | minimal design | Lets a client rewrite anyone's bytes. A removed Mac keeps it for an hour. Rests on the same unprobed list-under-JWT |
| Worker list **and delete** and title routes for recordings | security design | Widens RQ:414 further than needed. Delete and rename run on the Mac under a 15-minute `manage` credential instead |
| The Worker claims member metas (R10's v2 flow) | §16 #11 | Conflicts with the claim deferred to Stop (DV #4) and with N4's "bytes direct". The ledger gives the trust without the write path |
| `x-amz-meta-owner` on member-written metas as the Library's owner map | coverage design | Member-written, so forgeable. Also one more probe. The Worker-written ledger's `customMetadata` replaces it |
| The plaintext join code sent at invite creation | coverage design | The Worker never needs it before consume. A hash keeps pending codes out of Worker memory and logs |
| The Worker generates `k` | coverage design | App-made `k` with only its hash sent matches every other secret's shape |
| `GET /join/<code>` JSON preview | minimal design | Duplicates peek |
| A separate `GET /_shroom/parent` | coverage design | Folded into enroll's `stale-parent` |
| A space id (`sid`) in the roster, ping and `space.json` | coverage design | The host is permanent (A §8), and a fresh roster already reads `unauthorized`. One fewer field to keep in step |
| Cookie epochs and a sign-out-all-browsers bump | security design | Device-bound cookies already revoke per Mac. Cookie-key rotation stays a docs-level operation |
| 24-hour upload credentials with a 7-day post-removal grace rule | security design | More machinery than N4 and N32 need. Seven-day per-recording credentials make the copy true directly |
| Short pretty codes with an attempt limiter | — | Needs counters, which means writes on a failure path. 96-bit codes are pasted, never typed |
| Join returns a `k`, so the app opens a keyed page for "You're in" | §16 #54 | No spec'd moment opens a page at join. Best-effort instead (open question 5) |
| A web library on the Worker behind a cookie | A §13 | An enumerable index behind a cookie (already rejected) |

**Rejected in the critic's revision** — first-draft choices the review overturned, and
the alternatives it raised that this revision declined:

| Rejected | Finding | Why not |
|---|---|---|
| Re-placing `R2_PARENT` only on a wiring or repair pass, never on a routine `stale-parent` | H1, C6 | Left a fresh paste's Worker on the old (possibly deleted) parent until someone ran a repair; kept a deleted token's secret able to pass parent proof; made a live token Mac's flag read false and offer a Remove… a device key can't honor. Re-placing on every live-token `stale-parent` costs one secrets call per entry while two tokens live |
| The token-holder flag as `tokenKey == R2_PARENT.key_id` and a fresh `proofAt` | H1 | With the parent following the last-entering Mac, it flips with every entry. `proofAt` alone means "proved a live token within 30 days" |
| Relabelling a revived member with the new invite's label | H2 | A revive proves the joiner held the old device, not that the owner meant this invite for them; it would rename every old video in *Everyone* |
| Token Macs taking *Mine*, labels and "This is Mher's video." from `meta.owner` | H3 | Member-writable. The ledger map (`owners`, or a GET per uncached entry) is immutable and Worker- or token-written |
| A per-device upload budget counted inside the roster, or from a ledger list | H4 | A roster write per recording on the one hot key (C5), or a full list per first mint. A small counter object per device instead |
| An unconditional `k` grant PUT, and redeem by read-then-delete | H5, H8 | An unconditional PUT could overwrite another grant and accepted any key shape; read-then-delete let two concurrent redeems both mint a cookie. Create-only grants with a strict hex name, and a CAS spend |
| Accepting the concurrent-redeem race as named | H8 | The CAS costs one write on a rare path and rides P1, which is already a hard gate |
| `open "$APP" --args --join <link>` (B29 as built) | H9 | A live code in the app's argv for the app's whole life |
| The installer opening `shroom://join/…` instead | H9 | The scheme may not be registered for an app placed seconds earlier; the hand-off could fail silently |
| Pruning every invite 30 days after it ended or expired | U7 | An expired pending invite is still a row with [ New link ]; a used one backs the join replay and `join-used` |
| `join-waiting`'s "isn't answering" for a space whose page is too old | U3 | The space *is* answering; the spec has the line that says what will fix it |
| A Cancel row for pending own-Mac links, in v1 | U4 | The spec draws no such row. Proposed for Bagrat instead (open question 16) |
| `space-needs-owner` (or `space-gone`) on a single space-class answer | C7 | One R2 or DNS blip would flash an amber banner, or grey Start |
| Reseeding a broken roster automatically | C8 | It silently drops every member and pending invite; it waits on Bagrat (open question 19) |
| A cookie-checked `/_shroom/status/<id>` that carries "why it waits" | U1 | A roster read every two seconds per open owner page. The waiting words come with the render |

---

## 9. Open questions for Bagrat

*Status at `6a623b0`:* every recommendation below is built, provisionally, except where a
question says otherwise — #19's reseed (only the set-aside is built) and #16's pending
own-Mac row (left out, as recommended). Building one answers nothing: each is still his.

1. **§16 #3 — registry store.** *Recommend R2:* `.shroom/registry/roster.json` plus the
   ledger, with §2.1's reasons. This reopens R10's "never in the bucket" rule now that the
   bucket is private. It also rests on P1, now a hard gate: if R2's conditional put can't
   give one winner, the store comes back to you before any code. Choose the Durable
   Object and §2.5 applies, including a DEVIATIONS entry for the repair row that re-runs
   wiring and nothing else (SM:649-651).
2. **§16 #4b / RQ:414 — the Worker's boundary.** *Recommend:* the Worker gains reading
   (the roster, the ledger, one filtered Library list) and minting (`upload`, `manage`),
   and writes only records of its own (the roster, the ledger, `k` grants, per-device
   upload budgets, one advisory health flag). No delete of bytes and no meta claim.
   Renames and deletes run on the Mac under a 15-minute credential scoped to one
   recording. Confirm.
3. **§16 #11 — ownership.** *Recommend:* the Worker-written, create-only ledger (one write
   per recording); `meta.owner` becomes informational. The residual is the offline
   capture-start window (§3.7). Accept that residual, or add the device-bound id
   (rejected in §8)?
4. **B25 amended — a DECISIONS amendment, not a silent rewrite (C2).** Cookies become
   `{m,d,t}` with a live roster check: +1 read per cookie-bearing page view, and Remove…
   ends that browser's powers. Pre-change cookies stop granting owner-mode. The grant
   becomes `{member, device, exp}`, written by `POST /_shroom/api/key` for every Mac.
   **Fallback kept:** a token Mac whose page is older than iface 2 writes the grant
   directly over S3, as B25 does today, in a shape both pages read — so the setup Mac
   never loses owner-mode between an app update and its page refresh. A `broken` roster
   makes every cookie a recipient until repaired (fail closed). Confirm.
5. *(Built as recommended — DEVIATIONS #31.)* **§16 #54 — the join page's best-effort faces.** "You're in" and the owner view need a
   cookie a fresh joiner (or a fresh browser) doesn't have; `-elsewhere` rests on the
   user agent (§4.3). *Recommend:* accept all three as best-effort, via a DEVIATIONS
   entry for the two cookie faces.
6. **Arrival-time faces and offline arrivals (r6 note 3) — DEVIATIONS candidates.**
   - Peek knows at arrival that a link is dead (or its space missing). The spec reaches
     `join-expired`, `join-used` and `join-space-missing` only from `joining`.
     *Recommend:* show them at arrival.
   - **Offline arrival (U9):** `member-welcome` needs the inviter's name, which only peek
     supplies. *Recommend:* the window rests in `join-waiting`'s offline face ("You're
     offline — I'll finish joining the moment you're back.") until peek answers, then
     becomes `member-welcome`. That reaches `join-waiting` from arrival rather than from
     `joining`, and its line speaks of finishing a join nobody has clicked yet — a
     DEVIATIONS entry, or new copy for the arrival if you'd rather.
7. **Pending links can't be re-copied.** Only hashes exist after the born state, so the
   way back is [ New link ]. Fine for v1?
8. **Re-invite after removal (H2)** revives the same member id **and keeps its old
   label**; the new invite's label is only recorded. The waiting takes drain, *Mine*
   continues, and the owner's pending row becomes the returning member's row under the
   old name ("Mher — joined Oct 2") — even if the invite was typed for someone else,
   which is exactly the case worth seeing. Confirm.

   After 180 days the dead device is pruned, the Mac re-joins as a new member under the
   new label, and its old waiting takes stay local (Save to Movies).
9. **The token-holder flag is a parent proof within 30 days, nothing else (H1).** This
   fixes the permanently un-removable row a token Mac leaves after an offline
   Disconnect, and keeps a live token Mac's row un-removable whichever token is placed.
   The cost: a token Mac silent for 30 days shows Remove…. If removed, it re-enrolls
   itself on its next start (the spec's own reason a device key can't stop it). Accept,
   or keep the flag permanent and accept the stale row?
10. **Code length.** Links carry 16-character codes (96 bits), not the spec's illustrative
    six. Ratify as with B14.
11. **New copy, needs ratification:**
    - **(a)** The join page while the space's records can't be read: "This invite link
      doesn't work right now — try again a little later."
    - **(b)** The in-app wording for an unknown code, and for own-Mac join failures beyond
      the page's expired line (r6 note 3); and the own-Mac owner view's note without a
      name (§4.3).
    - **(c)** A cap hit (20 pending invites, 200 members): today the composer just keeps
      its text. Unreachable at v1's scale; add a line only if you want one.
    - **(d)** The removed owner Mac's banner (open question 18). *(Written — DEVIATIONS #21, the copy table.)*
    - **(e)** The pending own-Mac row and its confirm, if you want them (open question 16).
12. **Two token Macs with different tokens (H1) — the recommendation changed.** Each
    live-token Mac now re-places the Worker's parent with its own pair whenever enroll
    answers `stale-parent`, so the parent follows whichever entered last. Both flags stay
    true, neither row offers Remove…, and members mint under whichever live token is
    placed. The cost is one secrets call per entry while both tokens live; deleting the
    old token in Cloudflare (the docs say so when you paste a new one) ends it. The first
    draft's "no automatic re-placement" left a fresh paste's Worker on a dead parent and
    a deleted token's secret still able to prove. Confirm.
13. **Own-Mac links are the riskiest artifact** (§3.7): owner-class for an hour, named by
    the joiner, and — following the spec — not cancellable in v1 (U4, open question 16).
    N33 accepts this. A confirm on the issuing Mac ("MacBook Air wants to join") would be
    a new state. *Recommend:* not in v1; named here.
14. **Instant cut-off** stays parent-token rotation, docs-only, with no v1 UI (r6 flag 1).
    Confirm it is still the right scaffold call now that the Worker holds the parent —
    and note that rotation now needs a live-token Mac's next entry (or a repair) to
    re-place the parent before the old secret stops proving (§3.7).
15. *(Built as recommended, B41 n — DEVIATIONS #30.)* **The needs-update line during a join (U3) — a DEVIATIONS candidate.** When the
    joiner's app needs a newer page than the space has, this doc shows the spec's own
    line (members: "Bagrat's space needs an update — it happens when Bagrat updates
    shroom."; own-Mac: the owner-class line with [ Add a token here ]) in the invite
    window, resting until the page catches up. The spec places those lines on `config`;
    the invite window is new ground. *Recommend:* accept the placement. The alternative
    is `join-waiting`'s "isn't answering", which says something false.
16. **A row for a pending own-Mac link (U4)?** The spec draws none, so v1 has no way to
    cancel one; a leaked link is bounded by its hour and its one use, and shows up as an
    unexpected Mac. *Recommend:* leave it out of v1. If you want it: the nest stays under
    You while the link lives — "A new Mac — link good for 52 more minutes · Cancel…" —
    confirming with "**Cancel this link?**" / "The link stops working right away."
    [ Keep it ] ··· [ Cancel link ] (new copy; the route already exists).
17. **The mark on join pages (U6).** This doc shows "made with shroom" on every join-page
    face whatever the space's switch says, reading the switch's own line ("The small mark
    on every video page") literally: the join page isn't a video page, and it is the
    invitee's first sight of shroom. *Recommend:* keep it. Or let the switch govern join
    pages too (one more read per join page).
18. **A removed owner-class Mac has no face (U8) — a spec gap.** N34's faces are member
    faces; `member-revoked` on the owner's own other Mac would read "Your access to
    Bagrat's space was turned off… ask Bagrat if that's a surprise." *Recommend:* the same
    posture (banner under the amber dot, Start greyed, Save to Movies while takes wait)
    with "This Mac was removed from your space. Recordings on this Mac are safe — add it
    again from Members on your other Mac." · "Disconnect this Mac…". New copy.
19. **A broken roster (C8).** If `roster.json` stops parsing (realistically, a hand edit
    in the dashboard), every Region 6 route fails closed. The token Mac's repair pass
    sets the broken copy aside; reseeding brings back member #1 and that Mac at once, but
    **drops every member, every other owner Mac and every pending invite**, and members'
    old videos lose their labels and leave their *Mine*. *Recommend:* the repair pass asks
    before it reseeds, in the repair mood — which needs a line the spec doesn't have — or
    you prefer hand repair only (the space stays in the repair mood until the copy is
    fixed and put back). Which?
20. **The installer hands the join link over by file (H9; amends B29).** `install.sh`
    writes `~/.shroom/invite` (mode 600) and opens the app with `--invite`, instead of
    `--args --join <link>`, which kept a live code in the app's argv for its whole life
    (law 13). The ratified header doesn't change; step 4 grows a few lines. Confirm.
21. **New numbers** (scaffold flags; change freely): 20 new recordings per device per
    hour (the upload budget); `proofAt` restamped at most daily; space-side faces after
    two answers 30 s apart; record-less bytes reaped after 8 days.
22. **Still open, named, not designed here** (no call needed yet):
    - from §16 #44: unknown ids never reach "…stalled" (a page-load timer is proposed,
      Region 4's); which R2 binding errors mean *unavailable* (unrecorded); "Waiting for
      connection" can't tell offline from asleep (as built);
    - from §16 #49 / §5.5: who runs the sweep and on what trigger; multi-select delete
      (one `manage` mint per id, no batch, partial-failure rows unset); an offline
      Discard leaves record-less bytes for the sweep's 8-day rule.
