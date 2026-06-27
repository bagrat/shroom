# shroom uploader

The **file-by-file uploader** (SPEC §5). Speaks the raw S3 API with SigV4 so the
exact same path works against **R2 / B2 / AWS / MinIO** unchanged — *not*
`wrangler r2 object put`, which would silently re-lock to R2 (SPEC §9). Zero
dependencies: Node `crypto` + global `fetch`, path-style addressing.

## Core principle: the recording is sacred

ffmpeg writes segments to disk regardless of network. The uploader streams them up
opportunistically in the background and **never blocks recording**. If it falls
behind, finalize uploads the remainder. Deterministic keys (`<id>/<file>`) +
idempotent PUTs make retry/resume trivial — on restart, diff local files against
the bucket (HEAD) and upload only the gap.

- **What ships to the bucket:** `init.mp4`, `seg_NNNNN.m4s`, and `stream.m3u8`.
  HLS only. The progressive `preview.mp4` and the per-take intermediates
  (`preview_<k>.mp4`, `stream_<k>.m3u8`) are **local-only** and never uploaded.
- **Publish = the playlist, uploaded last.** `stream.m3u8` goes up only once every
  segment + init is confirmed present. No viewer ever sees a playlist that points
  at missing bytes (SPEC §5/§6). Uploading the playlist *is* the go-live act.

## Key/prefix scheme (SPEC §11, decided here)

`<id>/<file>`, where `<id>` is the recording's unguessable id (the bucket prefix).
Example: `AbC123xyz/seg_00007.m4s`, `AbC123xyz/stream.m3u8`.

## Credentials

The per-segment S3 keys live in `~/.shroom/credentials.json` (mode 600, never in
git — SPEC §9), or `SHROOM_S3_*` env vars (which override the file). Shape:

```json
{ "endpoint": "https://<acct>.r2.cloudflarestorage.com",
  "region": "auto", "bucket": "shroom",
  "accessKeyId": "…", "secretAccessKey": "…" }
```

## Usage

The recorder calls the `Uploader` inline during recording (enqueue per segment,
publish at finalize). For manual sync / **crash-resume**:

```bash
node upload.mjs <session-dir> [--id <id>]   # diff vs bucket, upload the gap, publish
```

## Layout

```
upload.mjs                 sync/resume CLI
lib/s3.mjs                 SigV4 signing + PUT/HEAD/GET (verified vs botocore)
lib/storage-config.mjs     load creds from ~/.shroom or SHROOM_S3_* env
lib/uploader.mjs           queue + backoff + gap-diff + publish-last gate
test/sigv4.test.mjs        known-answer test vs AWS botocore golden vectors
test/uploader.test.mjs     behaviour tests vs an in-process mock S3
test/mock-s3.mjs           the mock (PUT/HEAD/GET + failure injection)
```

## Tests

```bash
node test/sigv4.test.mjs       # SigV4 == botocore, byte for byte
node test/uploader.test.mjs    # idempotency, retry/backoff, publish gate, resume
```

Both run offline with no credentials and no Docker. The first **real R2** PUT is
deferred to M5 (provisioning) — but since our signature matches AWS's own signer,
R2 will accept it.
