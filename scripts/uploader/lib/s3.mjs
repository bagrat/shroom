// Minimal S3-compatible client: AWS Signature V4 signing + PUT/HEAD/GET.
//
// Deliberately speaks the raw S3 API (SigV4) rather than `wrangler r2 object put`,
// so the exact same upload path works against R2 / B2 / AWS / MinIO unchanged
// (SPEC §9). Zero dependencies — Node crypto + global fetch. Path-style addressing
// (`<endpoint>/<bucket>/<key>`), which every S3-compatible provider supports.
//
// The signer is verified byte-for-byte against AWS's own botocore (golden vectors
// in test/sigv4.test.mjs).

import crypto from 'node:crypto';

const ALGO = 'AWS4-HMAC-SHA256';
const sha256hex = (data) => crypto.createHash('sha256').update(data).digest('hex');
const hmac = (key, data) => crypto.createHmac('sha256', key).update(data).digest();

function deriveSigningKey(secret, dateStamp, region, service) {
  const kDate = hmac(Buffer.from('AWS4' + secret, 'utf8'), dateStamp);
  const kRegion = hmac(kDate, region);
  const kService = hmac(kRegion, service);
  return hmac(kService, 'aws4_request');
}

// e.g. 20260102T030405Z
function amzDateNow() {
  return new Date().toISOString().replace(/[-:]/g, '').replace(/\.\d{3}/, '');
}

// RFC 3986 encode one path segment (S3 keeps the '/' separators between segments).
function encodeSegment(seg) {
  return encodeURIComponent(seg).replace(
    /[!*'()]/g,
    (c) => '%' + c.charCodeAt(0).toString(16).toUpperCase(),
  );
}

// Sign a request. Returns the headers fetch needs, plus the intermediate strings
// (handy for debugging / the golden-vector test). `amzDate` is injectable for
// deterministic tests; otherwise it's "now".
export function signRequest({
  method, url, body = Buffer.alloc(0), region, service = 's3',
  accessKeyId, secretAccessKey, amzDate = amzDateNow(),
}) {
  const u = new URL(url);
  const dateStamp = amzDate.slice(0, 8);
  const buf = Buffer.isBuffer(body) ? body : Buffer.from(body);
  const payloadHash = sha256hex(buf);

  const canonicalUri = u.pathname.split('/').map(encodeSegment).join('/');
  const canonicalQuery = [...u.searchParams.entries()]
    .sort(([a], [b]) => (a < b ? -1 : a > b ? 1 : 0))
    .map(([k, v]) => `${encodeSegment(k)}=${encodeSegment(v)}`)
    .join('&');

  // We only ever sign host, x-amz-content-sha256, x-amz-date — matching botocore's
  // S3SigV4Auth for a request with no extra headers.
  const headers = { host: u.host, 'x-amz-content-sha256': payloadHash, 'x-amz-date': amzDate };
  const keys = Object.keys(headers).sort();
  const canonicalHeaders = keys.map((k) => `${k}:${headers[k]}\n`).join('');
  const signedHeaders = keys.join(';');

  const canonicalRequest = [
    method, canonicalUri, canonicalQuery, canonicalHeaders, signedHeaders, payloadHash,
  ].join('\n');
  const scope = `${dateStamp}/${region}/${service}/aws4_request`;
  const stringToSign = [ALGO, amzDate, scope, sha256hex(canonicalRequest)].join('\n');
  const signature = hmac(
    deriveSigningKey(secretAccessKey, dateStamp, region, service),
    stringToSign,
  ).toString('hex');
  const authorization = `${ALGO} Credential=${accessKeyId}/${scope}, SignedHeaders=${signedHeaders}, Signature=${signature}`;

  return {
    authorization,
    payloadHash,
    amzDate,
    signature,
    canonicalRequest,
    stringToSign,
    // Host is set by fetch from the URL (it's a forbidden header to set manually).
    fetchHeaders: {
      'x-amz-date': amzDate,
      'x-amz-content-sha256': payloadHash,
      authorization,
    },
  };
}

// A client = the per-segment upload credentials + target (SPEC §9).
//   { endpoint, region, bucket, accessKeyId, secretAccessKey }
export function objectUrl(client, key) {
  return `${client.endpoint.replace(/\/$/, '')}/${client.bucket}/${key}`;
}

async function send(client, method, key, body) {
  const url = objectUrl(client, key);
  const signed = signRequest({
    method, url, body: body ?? Buffer.alloc(0),
    region: client.region, accessKeyId: client.accessKeyId,
    secretAccessKey: client.secretAccessKey,
  });
  const res = await fetch(url, {
    method,
    headers: signed.fetchHeaders,
    body: method === 'PUT' ? body : undefined,
  });
  return res;
}

// PUT is idempotent: deterministic key + full-object write means a retry just
// rewrites identical bytes. Returns { ok, status, etag }.
export async function putObject(client, key, body) {
  const res = await send(client, 'PUT', key, body);
  return { ok: res.ok, status: res.status, etag: res.headers.get('etag') };
}

// HEAD to check existence (resume-gap diff). 200 = present, 404 = missing.
export async function headObject(client, key) {
  const res = await send(client, 'HEAD', key);
  return { exists: res.status === 200, status: res.status };
}

export async function getObject(client, key) {
  const res = await send(client, 'GET', key);
  if (!res.ok) return { ok: false, status: res.status, body: null };
  return { ok: true, status: res.status, body: Buffer.from(await res.arrayBuffer()) };
}
