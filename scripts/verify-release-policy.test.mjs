import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { mkdir, mkdtemp, readFile, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';
import test from 'node:test';

import { verifyReleasePolicy } from './verify-release-policy.mjs';

const validNotice = `# Aywa third-party notices

CPython embeddable distribution
get-pip.py
python.org
bootstrap.pypa.io
download.pytorch.org
huggingface.co
SBOM
`;

async function createFixture({ packageVersion = '0.1.0', cargoVersion = '0.1.0', tauriVersion = '0.1.0', targets = ['nsis'], author = 'Firas Jaber', security = true, checklist = true, checklistContent = 'unsigned\nSHA256SUMS.txt' } = {}) {
  const root = await mkdtemp(join(tmpdir(), 'aywa-policy-'));
  await mkdir(join(root, 'src-tauri'));
  await mkdir(join(root, 'docs'));
  await Promise.all([
    writeFile(join(root, 'LICENSE'), 'MIT License\nCopyright (c) 2026 Firas Jaber\n'),
    writeFile(join(root, 'NOTICE'), 'Aywa\nCopyright (c) 2026 Firas Jaber\n'),
    writeFile(join(root, 'THIRD_PARTY_NOTICES.md'), validNotice),
    writeFile(join(root, 'package.json'), JSON.stringify({ license: 'MIT', version: packageVersion })),
    writeFile(join(root, 'src-tauri', 'Cargo.toml'), `[package]\nversion = "${cargoVersion}"\nauthors = ["${author}"]\nlicense = "MIT"\nrepository = "https://github.com/firasjaber/aywa"\n`),
    writeFile(join(root, 'src-tauri', 'tauri.conf.json'), JSON.stringify({ version: tauriVersion, bundle: { targets, createUpdaterArtifacts: true }, plugins: { updater: {
      pubkey: Buffer.from('untrusted comment: minisign public key: TEST\nRWfixture\n').toString('base64'),
      endpoints: ['https://github.com/firasjaber/aywa/releases/latest/download/latest.json']
    } } })),
    security ? writeFile(join(root, 'SECURITY.md'), '# Security') : Promise.resolve(),
    checklist ? writeFile(join(root, 'docs', 'RELEASE_CHECKLIST.md'), checklistContent) : Promise.resolve()
  ]);
  return root;
}

test('accepts synchronized unsigned-release policy requirements', async (t) => {
  const root = await createFixture();
  t.after(() => rm(root, { recursive: true, force: true }));
  assert.deepEqual(await verifyReleasePolicy(root, { tag: 'v0.1.0' }), []);
});

test('CLI validates the supplied checkout and rejects mismatched or missing tags', async (t) => {
  const root = await createFixture();
  t.after(() => rm(root, { recursive: true, force: true }));
  const script = fileURLToPath(new URL('./verify-release-policy.mjs', import.meta.url));
  const run = (...args) => spawnSync(process.execPath, [script, ...args], { encoding: 'utf8' });
  assert.equal(run(root).status, 0);
  assert.equal(run(root, '--tag', 'v0.1.0').status, 0);
  const mismatch = run(root, '--tag', 'v99.0.0');
  assert.equal(mismatch.status, 1);
  assert.match(mismatch.stderr, /must match version 0\.1\.0/);
  const missing = run(root, '--tag');
  assert.equal(missing.status, 1);
  assert.match(missing.stderr, /--tag requires a release tag/);
  const packagePath = join(root, 'package.json');
  const metadata = JSON.parse(await readFile(packagePath, 'utf8'));
  metadata.version = '0.1.1';
  await writeFile(packagePath, JSON.stringify(metadata));
  const invalidCheckout = run(root);
  assert.equal(invalidCheckout.status, 1);
  assert.match(invalidCheckout.stderr, /must use the same version/);
});

test('rejects missing update signing and insecure feeds', async (t) => {
  const root = await createFixture();
  t.after(() => rm(root, { recursive: true, force: true }));
  const path = join(root, 'src-tauri/tauri.conf.json');
  const config = JSON.parse(await readFile(path, 'utf8'));
  config.bundle.createUpdaterArtifacts = false;
  config.plugins.updater.pubkey = '';
  config.plugins.updater.dangerousInsecureTransportProtocol = true;
  await writeFile(path, JSON.stringify(config));
  assert.deepEqual(await verifyReleasePolicy(root), [
    'tauri.conf.json must create updater artifacts.',
    'tauri.conf.json must contain the updater public key.',
    'tauri.conf.json must use the stable HTTPS update endpoint without allowing insecure transport or downgrades.'
  ]);
});

test('reports version, Windows target, documentation, author, and tag failures', async (t) => {
  const root = await createFixture({ cargoVersion: '0.1.1', targets: 'all', author: 'you', security: false, checklistContent: '# Releases' });
  t.after(() => rm(root, { recursive: true, force: true }));
  assert.deepEqual(await verifyReleasePolicy(root, { tag: 'v0.2.0' }), [
    'Cargo.toml must identify a non-placeholder author.',
    'package.json, Cargo.toml, and tauri.conf.json must use the same version.',
    'tauri.conf.json must target only the Windows NSIS bundle.',
    'SECURITY.md must document public release operations.',
    'docs/RELEASE_CHECKLIST.md must disclose that installers are unsigned.',
    'docs/RELEASE_CHECKLIST.md must document SHA256SUMS.txt verification.',
    'Release tag v0.2.0 must match version 0.1.1.'
  ]);
});
