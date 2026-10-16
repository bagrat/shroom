# Contributing to shroom

<!-- Placeholder wording until the approved copy lands. -->

Thanks for helping. shroom is a small project: a Mac app that records your screen and
shares it from your own Cloudflare account.

## Reporting a problem or an idea

[Open an issue](https://github.com/bagrat/shroom/issues/new/choose) and pick the kind.
Check the [known limits](https://shroom.am/faq) first: it may be one of them. Leave out
anything private (your Cloudflare token or account id, links to videos you don't want seen).
For anything you'd rather not post in public, write to hi@shroom.am.

## Changing code

Open an issue before a large change, so we can agree on it first. Then:

1. **The spec decides behaviour and words.** What shroom does and says, state by state, is
   [`spec/machine.json`](spec/machine.json) (readable as [`spec/MACHINE.md`](spec/MACHINE.md)).
   A change to behaviour or to a line a person reads starts there.
2. **The look is drawn before it's built.** A new or changed screen is proposed as a design
   first ([`docs/DESIGN.md`](docs/DESIGN.md)); describe it in the issue.
3. **Tests come with the change.** How to run them is in
   [`docs/TESTING-PLAN.md`](docs/TESTING-PLAN.md); the quick ones:

   ```sh
   npm ci
   npm test --workspace worker        # the video page
   npm run test:site                  # shroom.am
   npm run test:installer             # the install line
   cargo test -p shroom-core          # the core (any OS); capture and recording need a Mac
   ```

4. **One change per commit**, written so it reverts cleanly.

How the code is laid out: [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md).

## Licence

shroom is MIT-licensed ([`LICENSE`](LICENSE)). By contributing you agree your
contribution is under the same licence.
