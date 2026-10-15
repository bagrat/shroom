# The test space moves to its own account: D3, rewritten

*Proposal, 2026-10-14. **Ratified by Bagrat the same day, as written:** D3 in
TESTING-PLAN §9 now carries §2's text. The suite, the CI environment and the hosted-Mac
job follow, in that order.*

**In short.** D3 put the live tests on Bagrat's main account, fenced to `shroom-test-…`
names, because a second account couldn't be made. One exists now. The tests can move
there and stop being fenced. That makes the wizard's from-nothing paths testable live
(`name`, `name-taken`, wiring onto an empty account, `space-found`), and lets teardown
sweep the whole account. Main-account testing can then retire.

## 1. Where the account stands (probed 2026-10-14, read-only)

- Email verified. A card is on file and R2 is on: the bucket list answers 200, with no
  buckets in it. There are no Workers, and **no workers.dev name yet** (`10007`).
- Before the card, the same bucket list answered 403 `10042`. Those answers are recorded
  in `crates/shroom-cloud/tests/fixtures/`, and the L1 fake is held to them.
- The token `shroom-e2e-cloud` is account-owned: Workers Scripts Edit, Workers R2
  Storage Edit, **Account API Tokens Edit**. It lives at `~/.shroom-dev/e2e-account-token` (moved 2026-10-14 to the repo's gitignored `.secrets/`)
  (mode 600), and in cloud sessions in `SHROOM_E2E_TOKEN` (setup unsets it before its
  install scripts run).

## 2. D3 as proposed

> **D3 — Where the test space lives.** **A dedicated Cloudflare account** (test@shroom.am,
> 2026-10-14), holding nothing but tests. *Supersedes 2026-10-02: the main account,
> namespaced.* **Guards, enforced in the harness, not by convention:**
>
> - **Pinned.** A run starts only if its token verifies against the e2e account id and
>   that account's workers.dev name isn't `myshroom`. Pointed at the main account by
>   mistake, it stops before its first write.
> - **The product's powers.** The account token's only extra power, API Tokens Edit, is
>   used to mint one short-lived token per run with exactly the product's two powers.
>   The run deletes that token on the way out. The tests drive the product's token, so a
>   powers mistake fails here the way it would for a user.
> - **Swept.** Each run starts and ends by deleting every Worker and bucket on the account.
>   Everything there is the tests', so a crashed run can't poison the next.
> - **The workers.dev name is claimed once, by Bagrat's go.** The account's no-name state
>   exists only until the first claim, and it is recorded. After the claim, renames on
>   this account are fair game.
>
> **Consequence:** `name`, `name-taken` (claiming `myshroom`, which the main account
> holds), wiring from nothing, `space-found`, `space-storage-gone` and `name-changed`
> run live. `gate-email` and `gate-card` can't recur on this account (verified, card on);
> they stay on the L1 fakes, which are held to the recorded answers.

## 3. What it costs and what can go wrong

- **Money.** The card makes usage billable past R2's free tier. A run moves kilobytes to
  megabytes, so the free tier covers it many times over. A **billing alert** on the
  account (Bagrat's hands, once) makes a runaway loop visible.
- **The first `name` test is one-shot.** It turns the account's no-subdomain state into a
  named one for good. *Probed after the claim (2026-10-14): a named account refuses any
  further claim, `409 10036` “Account already has an associated subdomain.”, even for a
  name someone else holds. So there are no renames through the API, and `name-taken`
  (`10031`) only ever answers an account without a name. Both stay on the fakes; the
  fake now answers `10036` too.*
- **Token custody.** API Tokens Edit can mint tokens on this account only, and the
  account holds nothing real. That is the case for keeping the power: per-run product
  tokens are worth more than a narrower parent.

## 4. What changes when it's ratified

1. TESTING-PLAN §9 D3 is replaced by §2's text. H1 (symlinking the main-account token)
   and H5 (deleting `shroom-wizard-probe`) are reworded for the retirement.
2. **`tests/e2e-live/`**: the pinned, swept harness, then one test per Region 2 state
   §2 lists. The existing `*_live.rs` tests move onto the harness one by one, each green
   on the new account before the next moves.
3. **`cf-e2e` GitHub environment**: holds the token and the account id, limited to
   `main`, with Bagrat as required reviewer (his call: reviewer or not).
4. **The hosted-Mac heartbeat job**: runs the e2e suite with the app on a hosted Mac and
   keeps its video as an artifact.

## 5. Bagrat's calls — answered 2026-10-14

- D3: **ratified as written.**
- The workers.dev name: **`shroom-e2e`**, claimed once, after the suite's first
  no-name test records the state.
- Main-account testing: **retires** once the e2e suite is green.
- `cf-e2e`: **Bagrat as required reviewer** per run.
