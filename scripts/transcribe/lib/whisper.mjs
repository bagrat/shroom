// The whisper seam — the real `runWhisper` injected into lib/transcribe.mjs in
// production. Kept tiny and side-effect-only so the transcription core stays
// testable with a fake (tests never spawn whisper). whisper is slow (model load +
// decode), so we tee its progress to our stderr live, then read back the JSON it
// writes and hand the parsed object to the core.
//
// Invocation: `whisper <audio> --model <m> --output_format json --output_dir <dir>`.
// whisper names the output after the audio basename (e.g. preview.json); we resolve
// that path and parse it. Returns { ok, code, raw, jsonPath, stderr } — `raw` is the
// parsed whisper JSON (null on failure), so a missing binary / nonzero exit / no
// JSON all surface as ok:false rather than throwing.

import fs from 'node:fs';
import path from 'node:path';
import { spawn } from 'node:child_process';

export function spawnWhisper({ audioPath, outDir, model, bin = 'whisper', env = process.env, tee = true } = {}) {
  return new Promise((resolve) => {
    const args = [audioPath, '--model', model, '--output_format', 'json', '--output_dir', outDir];
    let stdout = '';
    let stderr = '';
    let child;
    try {
      child = spawn(bin, args, { env });
    } catch (e) {
      resolve({ ok: false, code: 127, raw: null, jsonPath: null, stderr: String(e?.message || e) });
      return;
    }
    child.stdout.on('data', (d) => { stdout += d; if (tee) process.stderr.write(d); });
    child.stderr.on('data', (d) => { stderr += d; if (tee) process.stderr.write(d); });
    child.on('error', (e) => resolve({ ok: false, code: 127, raw: null, jsonPath: null, stderr: stderr + String(e?.message || e) }));
    child.on('close', (code) => {
      if (code !== 0) {
        resolve({ ok: false, code: code ?? 1, raw: null, jsonPath: null, stderr });
        return;
      }
      const base = path.basename(audioPath, path.extname(audioPath));
      const jsonPath = path.join(outDir, `${base}.json`);
      try {
        const raw = JSON.parse(fs.readFileSync(jsonPath, 'utf8'));
        resolve({ ok: true, code: 0, raw, jsonPath, stderr });
      } catch (e) {
        resolve({ ok: false, code: 0, raw: null, jsonPath, reason: 'no_json', stderr: stderr + `\ncould not read ${jsonPath}: ${e.message}` });
      }
    });
  });
}
