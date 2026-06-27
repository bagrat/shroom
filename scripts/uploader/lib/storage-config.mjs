// Load the per-segment upload credentials (SPEC §9). These are the S3-compatible
// API keys (R2 access key / secret), kept in ~/.shroom/credentials.json (mode 600,
// never in the git repo). Env vars override the file — handy for tests / CI / a
// local MinIO. Shape:
//   { endpoint, region, bucket, accessKeyId, secretAccessKey }

import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';

export const DEFAULT_CREDS_PATH = path.join(os.homedir(), '.shroom', 'credentials.json');

export function loadStorageConfig({ credsPath = DEFAULT_CREDS_PATH, env = process.env } = {}) {
  let fromFile = {};
  if (fs.existsSync(credsPath)) {
    try {
      fromFile = JSON.parse(fs.readFileSync(credsPath, 'utf8'));
    } catch (e) {
      throw new Error(`Could not parse ${credsPath}: ${e.message}`);
    }
  }
  const fromEnv = {
    endpoint: env.SHROOM_S3_ENDPOINT,
    region: env.SHROOM_S3_REGION,
    bucket: env.SHROOM_S3_BUCKET,
    accessKeyId: env.SHROOM_S3_ACCESS_KEY_ID,
    secretAccessKey: env.SHROOM_S3_SECRET_ACCESS_KEY,
  };
  return {
    region: 'auto', // R2 default
    ...fromFile,
    ...Object.fromEntries(Object.entries(fromEnv).filter(([, v]) => v != null)),
  };
}

export function isConfigured(cfg) {
  return Boolean(cfg && cfg.endpoint && cfg.bucket && cfg.accessKeyId && cfg.secretAccessKey);
}

export function missingFields(cfg) {
  return ['endpoint', 'bucket', 'accessKeyId', 'secretAccessKey'].filter((k) => !cfg?.[k]);
}
