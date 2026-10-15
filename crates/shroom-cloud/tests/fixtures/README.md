# Recorded Cloudflare responses

Raw answers from the dedicated test account, captured 2026-10-14 with read-only GETs —
the same calls `shroom-cloud` makes. Each file is `{http_status, body}`, `body` holding
Cloudflare's bytes as sent; the token id is scrubbed to `<TOKEN_ID>` (no account id,
email or name appeared in these); the account id is scrubbed to `<ACCOUNT_ID>` wherever
it would appear.

- `bare-account/` — email verified, no card, no workers.dev subdomain:
  `verify.json` (`GET /accounts/{id}/tokens/verify`), `r2-buckets.json`
  (`GET /accounts/{id}/r2/buckets` → `10042`, `gate-card`), `subdomain.json`
  (`GET /accounts/{id}/workers/subdomain` → `10007`, `name`).
- `card-on/` — after the card was added on the R2 page (the page the wizard's
  Add credit card opens): `r2-buckets.json` → an empty list.
- `named/` — after `shroom-e2e` was claimed: `subdomain.json` (the GET answers the name)
  and `claim-another.json` (`PUT /accounts/{id}/workers/subdomain` asking for
  `myshroom` → 409 `10036`: a named account refuses any further claim).

`tests/fixtures_fake.rs` holds the L1 fake to these: status, `success` and error codes
exactly, result keys exactly; messages are Cloudflare's copy and aren't compared.
