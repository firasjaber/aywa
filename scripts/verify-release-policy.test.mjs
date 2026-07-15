import assert from 'node:assert/strict';
import { mkdir, mkdtemp, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
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

async function createFixture({ packageVersion = '0.1.0', cargoVersion = '0.1.0', tauriVersion = '0.1.0', targets = ['nsis'], author = 'Firas Jaber', security = true, checklist = true } = {}) {
  const root = await mkdtemp(join(tmpdir(), 'aywa-policy-'));
  await mkdir(join(root, 'src-tauri'));
  await mkdir(join(root, 'docs'));
  await Promise.all([
    writeFile(join(root, 'LICENSE'), 'MIT License\nCopyright (c) 2026 Firas Jaber\n'),
    writeFile(join(root, 'NOTICE'), 'Aywa\nCopyright (c) 2026 Firas Jaber\n'),
    writeFile(join(root, 'THIRD_PARTY_NOTICES.md'), validNotice),
    writeFile(join(root, 'package.json'), JSON.stringify({ license: 'MIT', version: packageVersion })),
    writeFile(join(root, 'src-tauri', 'Cargo.toml'), `[package]\nversion = "${cargoVersion}"\nauthors = ["${author}"]\nlicense = "MIT"\nrepository = "https://github.com/firasjaber/aywa"\n`),
    writeFile(join(root, 'src-tauri', 'tauri.conf.json'), JSON.stringify({ version: tauriVersion, bundle: { targets } })),
    security ? writeFile(join(root, 'SECURITY.md'), '# Security') : Promise.resolve(),
    checklist ? writeFile(join(root, 'docs', 'RELEASE_CHECKLIST.md'), '# Releases') : Promise.resolve()
  ]);
  return root;
}

test('accepts synchronized signed-release policy requirements', async (t) => {
  const root = await createFixture();
  t.after(() => rm(root, { recursive: true, force: true }));
  assert.deepEqual(await verifyReleasePolicy(root, { tag: 'v0.1.0' }), []);
});

test('reports version, Windows target, documentation, author, and tag failures', async (t) => {
  const root = await createFixture({ cargoVersion: '0.1.1', targets: 'all', author: 'you', security: false, checklist: false });
  t.after(() => rm(root, { recursive: true, force: true }));
  assert.deepEqual(await verifyReleasePolicy(root, { tag: 'v0.2.0' }), [
    'Cargo.toml must identify a non-placeholder author.',
    'package.json, Cargo.toml, and tauri.conf.json must use the same version.',
    'tauri.conf.json must target only the Windows NSIS bundle.',
    'SECURITY.md must document public release operations.',
    'docs/RELEASE_CHECKLIST.md must document public release operations.',
    'Release tag v0.2.0 must match version 0.1.1.'
  ]);
});
