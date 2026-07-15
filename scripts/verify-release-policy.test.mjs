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

async function createFixture({ packageLicense = 'MIT', cargoLicense = 'MIT', notice = validNotice } = {}) {
  const root = await mkdtemp(join(tmpdir(), 'aywa-policy-'));
  await mkdir(join(root, 'src-tauri'));
  await Promise.all([
    writeFile(join(root, 'LICENSE'), 'MIT License\nCopyright (c) 2026 Firas Jaber\n'),
    writeFile(join(root, 'NOTICE'), 'Aywa\nCopyright (c) 2026 Firas Jaber\n'),
    writeFile(join(root, 'THIRD_PARTY_NOTICES.md'), notice),
    writeFile(join(root, 'package.json'), JSON.stringify({ license: packageLicense })),
    writeFile(join(root, 'src-tauri', 'Cargo.toml'), `[package]\nlicense = "${cargoLicense}"\nrepository = "https://github.com/firasjaber/aywa"\n`)
  ]);
  return root;
}

test('accepts the required MIT metadata and policy content', async (t) => {
  const root = await createFixture();
  t.after(() => rm(root, { recursive: true, force: true }));
  assert.deepEqual(await verifyReleasePolicy(root), []);
});

test('reports missing or inconsistent release-policy requirements', async (t) => {
  const root = await createFixture({ packageLicense: 'Apache-2.0', notice: '# Incomplete' });
  t.after(() => rm(root, { recursive: true, force: true }));
  assert.deepEqual(await verifyReleasePolicy(root), [
    'package.json must declare license "MIT".',
    'THIRD_PARTY_NOTICES.md must mention "CPython embeddable distribution".',
    'THIRD_PARTY_NOTICES.md must mention "get-pip.py".',
    'THIRD_PARTY_NOTICES.md must mention "python.org".',
    'THIRD_PARTY_NOTICES.md must mention "bootstrap.pypa.io".',
    'THIRD_PARTY_NOTICES.md must mention "download.pytorch.org".',
    'THIRD_PARTY_NOTICES.md must mention "huggingface.co".',
    'THIRD_PARTY_NOTICES.md must mention "SBOM".'
  ]);
});
