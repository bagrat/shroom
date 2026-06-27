// Behaviour tests for the deploy step against a fake wrangler (no network, no
// Cloudflare account, no wrangler binary). Covers URL parsing, the production-base
// derivation, hls.min.js placement, the happy path + event emission, and failure
// surfaces. Run: node scripts/deploy/test/deploy.test.mjs

import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import assert from 'node:assert/strict';

import {
  parseDeploymentUrl,
  productionBaseFromDeployment,
  ensureHlsJs,
  runDeploy,
} from '../lib/deploy.mjs';

const ID = 'AbC123xyz';

// A realistic chunk of wrangler pages deploy output.
const WRANGLER_OK = `🌍  Uploading... (3/3)
✨ Success! Uploaded 3 files (1.23 sec)
✨ Deployment complete! Take a peek over at https://a1b2c3d4.shroom-site.pages.dev
`;

function tmpSite({ withPage = true, withHlsAtRoot = false } = {}) {
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'shroom-deploy-'));
  if (withPage) {
    fs.mkdirSync(path.join(dir, ID), { recursive: true });
    fs.writeFileSync(path.join(dir, ID, 'index.html'), '<html></html>');
  }
  if (withHlsAtRoot) fs.writeFileSync(path.join(dir, 'hls.min.js'), 'EXISTING');
  return dir;
}

function tmpVendor(present = true) {
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'shroom-vendor-'));
  const p = path.join(dir, 'hls.min.js');
  if (present) fs.writeFileSync(p, 'HLSJS-BYTES');
  return p;
}

// Fake wrangler: records the args it was called with, returns a canned result.
function fakeWrangler(result) {
  const calls = [];
  const fn = async (args) => { calls.push(args); return result; };
  fn.calls = calls;
  return fn;
}

function collector() {
  const events = [];
  const log = (event, fields) => events.push({ event, ...fields });
  log.events = events;
  log.find = (e) => events.find((x) => x.event === e);
  return log;
}

let passed = 0;
const tests = [];
const test = (name, fn) => tests.push([name, fn]);

test('parseDeploymentUrl picks the last pages.dev URL and trims punctuation', () => {
  assert.equal(parseDeploymentUrl(WRANGLER_OK), 'https://a1b2c3d4.shroom-site.pages.dev');
  assert.equal(parseDeploymentUrl('see https://x.proj.pages.dev.'), 'https://x.proj.pages.dev');
  assert.equal(parseDeploymentUrl('no url here'), null);
  // last wins when several are printed
  assert.equal(
    parseDeploymentUrl('alias https://proj.pages.dev then https://hash.proj.pages.dev'),
    'https://hash.proj.pages.dev',
  );
});

test('productionBaseFromDeployment strips the deployment hash label', () => {
  assert.equal(productionBaseFromDeployment('https://a1b2c3d4.shroom-site.pages.dev'), 'https://shroom-site.pages.dev');
  // already-bare production URL passes through
  assert.equal(productionBaseFromDeployment('https://shroom-site.pages.dev'), 'https://shroom-site.pages.dev');
  assert.equal(productionBaseFromDeployment(null), null);
});

test('ensureHlsJs copies from vendor when missing at site root', () => {
  const site = tmpSite();
  const vendor = tmpVendor(true);
  const r = ensureHlsJs({ siteDir: site, vendorPath: vendor });
  assert.equal(r.ok, true);
  assert.equal(r.placed, true);
  assert.equal(fs.readFileSync(path.join(site, 'hls.min.js'), 'utf8'), 'HLSJS-BYTES');
});

test('ensureHlsJs skips when already present at site root', () => {
  const site = tmpSite({ withHlsAtRoot: true });
  const vendor = tmpVendor(true);
  const r = ensureHlsJs({ siteDir: site, vendorPath: vendor });
  assert.equal(r.placed, false);
  assert.equal(fs.readFileSync(path.join(site, 'hls.min.js'), 'utf8'), 'EXISTING');
});

test('ensureHlsJs errors with the fetch command when nothing is vendored', () => {
  const site = tmpSite();
  const vendor = tmpVendor(false);
  const r = ensureHlsJs({ siteDir: site, vendorPath: vendor });
  assert.equal(r.ok, false);
  assert.equal(r.reason, 'no_vendor');
  assert.match(r.message, /fetch-hls\.mjs/);
});

test('runDeploy happy path: places hls.js, deploys, emits deployed + published', async () => {
  const site = tmpSite();
  const vendor = tmpVendor(true);
  const log = collector();
  const wrangler = fakeWrangler({ code: 0, stdout: WRANGLER_OK, stderr: '' });

  const r = await runDeploy({
    siteDir: site, projectName: 'shroom-site', id: ID,
    pageConfig: {}, vendorPath: vendor, runWrangler: wrangler, log,
  });

  assert.equal(r.ok, true);
  assert.equal(r.deploymentUrl, 'https://a1b2c3d4.shroom-site.pages.dev');
  // no pagesBaseUrl configured → derive production base from the deployment URL
  assert.equal(r.playbackUrl, 'https://shroom-site.pages.dev/AbC123xyz/');
  assert.equal(log.find('hlsjs_placed')?.path, path.join(site, 'hls.min.js'));
  assert.ok(log.find('deployed'));
  assert.equal(log.find('published').playbackUrl, 'https://shroom-site.pages.dev/AbC123xyz/');
  // deploy the whole site bundle to the production branch
  const args = wrangler.calls[0];
  assert.deepEqual(args.slice(0, 3), ['pages', 'deploy', site]);
  assert.ok(args.includes('--project-name=shroom-site'));
  assert.ok(args.includes('--branch=main'));
});

test('runDeploy prefers a configured pagesBaseUrl over the derived base', async () => {
  const site = tmpSite();
  const vendor = tmpVendor(true);
  const log = collector();
  const r = await runDeploy({
    siteDir: site, projectName: 'shroom-site', id: ID,
    pageConfig: { pagesBaseUrl: 'https://watch.example.com/' },
    vendorPath: vendor,
    runWrangler: fakeWrangler({ code: 0, stdout: WRANGLER_OK, stderr: '' }),
    log,
  });
  assert.equal(r.playbackUrl, 'https://watch.example.com/AbC123xyz/');
});

test('runDeploy fails (no published) when wrangler exits non-zero', async () => {
  const site = tmpSite();
  const vendor = tmpVendor(true);
  const log = collector();
  const r = await runDeploy({
    siteDir: site, projectName: 'shroom-site', id: ID,
    pageConfig: {}, vendorPath: vendor,
    runWrangler: fakeWrangler({ code: 1, stdout: '', stderr: 'Error: project not found' }),
    log,
  });
  assert.equal(r.ok, false);
  assert.equal(r.reason, 'wrangler_failed');
  assert.equal(log.find('published'), undefined);
  assert.match(log.find('deploy_failed').stderr, /project not found/);
});

test('runDeploy fails when wrangler succeeds but prints no URL', async () => {
  const site = tmpSite();
  const vendor = tmpVendor(true);
  const r = await runDeploy({
    siteDir: site, projectName: 'shroom-site', id: ID,
    pageConfig: {}, vendorPath: vendor,
    runWrangler: fakeWrangler({ code: 0, stdout: 'done, somehow, no url', stderr: '' }),
    log: collector(),
  });
  assert.equal(r.ok, false);
  assert.equal(r.reason, 'wrangler_failed');
});

test('runDeploy guards a missing page bundle before touching wrangler', async () => {
  const site = tmpSite({ withPage: false });
  const vendor = tmpVendor(true);
  const wrangler = fakeWrangler({ code: 0, stdout: WRANGLER_OK, stderr: '' });
  const r = await runDeploy({
    siteDir: site, projectName: 'shroom-site', id: ID,
    pageConfig: {}, vendorPath: vendor, runWrangler: wrangler, log: collector(),
  });
  assert.equal(r.ok, false);
  assert.equal(r.reason, 'page_missing');
  assert.equal(wrangler.calls.length, 0);
});

test('runDeploy without an id deploys but emits no published event', async () => {
  const site = tmpSite();
  const vendor = tmpVendor(true);
  const log = collector();
  const r = await runDeploy({
    siteDir: site, projectName: 'shroom-site',
    pageConfig: {}, vendorPath: vendor,
    runWrangler: fakeWrangler({ code: 0, stdout: WRANGLER_OK, stderr: '' }),
    log,
  });
  assert.equal(r.ok, true);
  assert.equal(r.playbackUrl, null);
  assert.ok(log.find('deployed'));
  assert.equal(log.find('published'), undefined);
});

for (const [name, fn] of tests) {
  try {
    await fn();
    passed++;
    process.stdout.write(`ok   ${name}\n`);
  } catch (e) {
    process.stdout.write(`FAIL ${name}\n  ${e?.stack || e}\n`);
    process.exitCode = 1;
  }
}
process.stdout.write(`\n${passed}/${tests.length} passed\n`);
