# shroom

**Record. Share. Pay-as-you-go.** Your videos live in your own Cloudflare
account — we never hold them. You set Cloudflare up once, in a guided setup — about five
minutes. You only ever pay Cloudflare. No account with us, no subscription.

```sh
curl -fsSL https://shroom.am/install | sh
```

Read it first — the same address opens as readable source. For macOS 14 or newer, Apple
silicon. More at [shroom.am](https://shroom.am).

## In this repo

- [`spec/MACHINE.md`](spec/MACHINE.md) — what shroom does and says, state by state (generated from [`spec/machine.json`](spec/machine.json), the spec).
- [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md) — how it's built; [`docs/`](docs/) has the rest:
  decisions, testing, releasing.
- [`CLAUDE.md`](CLAUDE.md) — how work on shroom is done, and where everything lives.
- [`LICENSE`](LICENSE) (MIT) and [`THIRD-PARTY-NOTICES.md`](THIRD-PARTY-NOTICES.md).

Trouble, questions: [open an issue](https://github.com/bagrat/shroom/issues/new).
