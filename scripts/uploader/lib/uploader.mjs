// The file-by-file uploader (SPEC §5) — the fail-safe core principle is that the
// recording is sacred and never blocks on upload. ffmpeg keeps writing segments to
// disk regardless of network; this uploader retries in the background with backoff;
// if it never catches up, finalize uploads the remainder.
//
// Deterministic keys (`<id>/<file>`) + idempotent PUTs make retry/resume trivial:
// on restart we diff local files against the bucket (HEAD) and upload only the gap.
// The master playlist (stream.m3u8) is uploaded LAST and only once every segment is
// confirmed up — uploading the playlist IS the "go live" act (SPEC §5/§6).

import fs from 'node:fs/promises';
import path from 'node:path';
import { putObject, headObject } from './s3.mjs';

const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

// Files that belong in the bucket: HLS only. The progressive preview.mp4 and the
// per-take intermediates (preview_<k>.mp4, stream_<k>.m3u8) are local-only.
export const PLAYLIST = 'stream.m3u8';
export const INIT = 'init.mp4';
const SEGMENT_RE = /^seg_\d+\.m4s$/;

export class Uploader {
  constructor(client, { id, dir, log = () => {}, maxAttempts = 6, baseDelayMs = 500, maxDelayMs = 15000 }) {
    this.client = client;
    this.id = id;
    this.dir = dir;
    this.log = log;
    this.maxAttempts = maxAttempts;
    this.baseDelayMs = baseDelayMs;
    this.maxDelayMs = maxDelayMs;

    this.queue = [];
    this.pending = new Set();   // queued or in-flight
    this.confirmed = new Set(); // PUT succeeded
    this.failed = new Map();    // file -> last error, after exhausting attempts
    this.running = false;
    this.worker = Promise.resolve();
  }

  keyFor(file) {
    return `${this.id}/${file}`;
  }

  // Opportunistic, best-effort: queue a freshly-written file for background upload.
  enqueue(file) {
    if (this.confirmed.has(file) || this.pending.has(file)) return;
    this.pending.add(file);
    this.queue.push(file);
    this.#kick();
  }

  #kick() {
    if (this.running) return;
    this.running = true;
    this.worker = this.#drainQueue();
  }

  async #drainQueue() {
    while (this.queue.length) {
      await this.#uploadWithRetry(this.queue.shift());
    }
    this.running = false;
  }

  // Wait for the background worker to go idle.
  async settle() {
    while (this.running) await this.worker;
  }

  async #uploadWithRetry(file) {
    let body;
    try {
      body = await fs.readFile(path.join(this.dir, file));
    } catch (e) {
      this.pending.delete(file);
      this.failed.set(file, `read: ${e.message}`);
      this.log('upload_failed', { file, message: `read: ${e.message}` });
      return false;
    }

    for (let attempt = 1; ; attempt++) {
      try {
        const res = await putObject(this.client, this.keyFor(file), body);
        if (res.ok) {
          this.confirmed.add(file);
          this.pending.delete(file);
          this.failed.delete(file);
          this.log('segment_uploaded', { file, key: this.keyFor(file), status: res.status, attempt });
          return true;
        }
        throw new Error(`HTTP ${res.status}`);
      } catch (e) {
        if (attempt >= this.maxAttempts) {
          this.pending.delete(file);
          this.failed.set(file, e.message);
          // The recording is sacred — never throw into the record path; finalize retries.
          this.log('upload_failed', { file, attempts: attempt, message: e.message });
          return false;
        }
        const delay = Math.min(this.maxDelayMs, this.baseDelayMs * 2 ** (attempt - 1));
        this.log('upload_retry', { file, attempt, nextDelayMs: delay, message: e.message });
        await sleep(delay);
      }
    }
  }

  // Gap-aware: ensure each file is in the bucket, skipping ones already confirmed
  // locally or already present remotely (HEAD). Used by finalize and by resume.
  async ensureUploaded(files) {
    for (const file of files) {
      if (this.confirmed.has(file)) continue;
      const { exists } = await headObject(this.client, this.keyFor(file));
      if (exists) {
        this.confirmed.add(file);
        this.log('already_present', { file, key: this.keyFor(file) });
        continue;
      }
      await this.#uploadWithRetry(file);
    }
  }

  // Finalize: drain the background queue, ensure every segment + init is up, then
  // publish the playlist LAST (the go-live act). Publishing is gated on a fully
  // uploaded session — no viewer ever sees a playlist that points at missing bytes.
  async finalizePublish({ segments, init = INIT, playlist = PLAYLIST }) {
    await this.settle();
    await this.ensureUploaded([init, ...segments]);

    const bytesUp = [init, ...segments].every((f) => this.confirmed.has(f));
    if (!bytesUp) {
      return { published: false, confirmed: [...this.confirmed], failed: [...this.failed.keys()] };
    }
    const ok = await this.#uploadWithRetry(playlist);
    this.log('published', { id: this.id, playlistKey: this.keyFor(playlist), ok });
    return { published: ok, confirmed: [...this.confirmed], failed: [...this.failed.keys()] };
  }

  // Standalone sync / crash-resume: discover the HLS files on disk, upload the gap,
  // and publish. Idempotent — safe to re-run.
  async syncDir() {
    const entries = await fs.readdir(this.dir);
    const segments = entries.filter((f) => SEGMENT_RE.test(f)).sort();
    const init = entries.includes(INIT) ? [INIT] : [];
    const hasPlaylist = entries.includes(PLAYLIST);

    await this.ensureUploaded([...init, ...segments]);
    const bytesUp = [...init, ...segments].every((f) => this.confirmed.has(f));

    let published = false;
    if (hasPlaylist && bytesUp) {
      published = await this.#uploadWithRetry(PLAYLIST);
      this.log('published', { id: this.id, playlistKey: this.keyFor(PLAYLIST), ok: published });
    }
    return {
      segments: segments.length,
      confirmed: [...this.confirmed],
      failed: [...this.failed.keys()],
      published,
    };
  }
}
