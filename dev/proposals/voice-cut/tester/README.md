# The “shroom cut” tester

A private page for trying the voice cue with a real voice, from a phone or a laptop, with
nothing to install. It runs as a POC Worker on the Cloudflare **test account**, behind a
throwaway key in the link's `#k=` fragment.

- **Record**, or **pick an audio or video file**, up to 5 minutes.
- The page turns the sound into 16 kHz mono WAV and finds the silences.
- `POST /listen` runs it through Whisper on Workers AI (`whisper-large-v3-turbo`), behind the
  key. Nothing is stored.
- The page runs the real planner (`crates/shroom-edit`, compiled to WebAssembly by `wasm/`)
  on the words.
- It shows the transcript with the cuts struck through, and the cuts with Undo or “Cut it
  too”, and plays the take with the cuts skipped.

```sh
rustup target add wasm32-unknown-unknown            # once
SHROOM_E2E_TOKEN=… SHROOM_E2E_ACCOUNT_ID=… dev/proposals/voice-cut/tester/deploy.sh       # prints the link
SHROOM_E2E_TOKEN=… SHROOM_E2E_ACCOUNT_ID=… dev/proposals/voice-cut/tester/deploy.sh down  # deletes it
```

Each deploy makes a new key, and the old link stops working.
