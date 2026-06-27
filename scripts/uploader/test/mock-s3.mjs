// A tiny in-process S3-compatible server for behaviour tests (no Docker, no creds,
// no network beyond loopback). Implements just PUT / HEAD / GET on `/<bucket>/<key>`
// with an in-memory store, plus failure injection to exercise retry/backoff.

import http from 'node:http';

export function startMockS3({ failuresByKey = {} } = {}) {
  const store = new Map(); // key -> Buffer
  const puts = [];         // log of every PUT key (to assert ordering / idempotency)
  const remainingFailures = new Map(Object.entries(failuresByKey));

  const server = http.createServer((req, res) => {
    // path is /<bucket>/<key...> — strip the leading bucket segment.
    const url = new URL(req.url, 'http://localhost');
    const parts = url.pathname.replace(/^\/+/, '').split('/');
    const key = parts.slice(1).join('/');

    // Every request must carry a SigV4 Authorization header.
    const auth = req.headers['authorization'] || '';
    if (!auth.startsWith('AWS4-HMAC-SHA256')) {
      res.writeHead(403).end('missing/invalid Authorization');
      return;
    }

    if (req.method === 'PUT') {
      const left = remainingFailures.get(key) || 0;
      if (left > 0) {
        remainingFailures.set(key, left - 1);
        res.writeHead(503).end('injected failure');
        return;
      }
      const chunks = [];
      req.on('data', (c) => chunks.push(c));
      req.on('end', () => {
        store.set(key, Buffer.concat(chunks));
        puts.push(key);
        res.writeHead(200, { etag: '"' + key.length.toString(16) + '"' }).end();
      });
      return;
    }
    if (req.method === 'HEAD') {
      res.writeHead(store.has(key) ? 200 : 404).end();
      return;
    }
    if (req.method === 'GET') {
      if (!store.has(key)) { res.writeHead(404).end(); return; }
      res.writeHead(200).end(store.get(key));
      return;
    }
    res.writeHead(405).end();
  });

  return new Promise((resolve) => {
    server.listen(0, '127.0.0.1', () => {
      const { port } = server.address();
      resolve({
        store,
        puts,
        endpoint: `http://127.0.0.1:${port}`,
        close: () => new Promise((r) => server.close(r)),
      });
    });
  });
}
