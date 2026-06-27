# vendor/ — bundled hls.js

Non-Safari browsers need [hls.js](https://github.com/video-dev/hls.js) (~100 KB,
one file, no build) to play HLS; Safari/iOS play it natively (SPEC §6). We
**self-host** it rather than hot-linking a CDN at playback time: that keeps the
player dependency-free and leaks no viewer IPs to a third party — the whole
self-host story.

## Not committed, not fetched silently

`hls.min.js` is **not** checked into this repo. `fetch-hls.mjs` pulls a **pinned
version** and verifies its **SHA-256** before writing — so what lands on disk is
reproducible and tamper-evident, and the fetch is an explicit step (run at setup,
M5, or by hand), never a silent `npm install` side effect.

```sh
node scripts/page/vendor/fetch-hls.mjs           # → vendor/hls.min.js
node scripts/page/vendor/fetch-hls.mjs --out /path/to/site/hls.min.js
```

First run prints the actual hash (the pin ships empty — we don't ship an
unverified hash); paste it into `PIN.sha256` in `fetch-hls.mjs` and re-run to
confirm it verifies. To bump versions, repeat with the new `version` + `url`.

At deploy (M5) this file is placed **once** at the Pages site root (`/hls.min.js`),
shared by every per-video page — so the 20k-file Pages limit counts it once, not
per video.
