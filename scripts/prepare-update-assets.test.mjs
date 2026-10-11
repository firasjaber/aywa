import assert from 'node:assert/strict';
import { cp, mkdir, mkdtemp, readFile, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import test from 'node:test';
import { prepareUpdateAssets } from './prepare-update-assets.mjs';

const project = resolve(dirname(fileURLToPath(import.meta.url)), '..');
async function fixture(t) {
  const root = await mkdtemp(join(tmpdir(), 'aywa-update-assets-'));
  t.after(() => rm(root, { recursive: true, force: true }));
  for (const file of ['LICENSE', 'NOTICE', 'THIRD_PARTY_NOTICES.md', 'package.json', 'src-tauri/Cargo.toml', 'src-tauri/tauri.conf.json', 'docs/RELEASE_CHECKLIST.md', 'SECURITY.md']) {
    await mkdir(dirname(join(root, file)), { recursive: true });
    await cp(join(project, file), join(root, file));
  }
  const { version } = JSON.parse(await readFile(join(root, 'package.json'), 'utf8'));
  const installerDir = join(root, 'installers');
  await mkdir(installerDir);
  const name = `Aywa_${version}_x64-setup.exe`;
  await writeFile(join(installerDir, name), 'installer fixture');
  await cp(join(project, 'src-tauri/test-data/update-fixture.txt.sig'), join(installerDir, `${name}.sig`));
  return { root, installerDir, name, tag: `v${version}` };
}

test('generates the exact release URL, signature, and checksum asset set', async (t) => {
  const { root, installerDir, name, tag } = await fixture(t);
  const metadata = await prepareUpdateAssets(root, { installerDir, tag });
  assert.equal(metadata.version, tag.slice(1));
  assert.equal(metadata.platforms['windows-x86_64'].url, `https://github.com/firasjaber/aywa/releases/download/${tag}/${name}`);
  assert.ok(metadata.platforms['windows-x86_64'].signature);
  assert.equal(await readFile(join(root, 'release-assets', `${name}.sig`), 'utf8'), metadata.platforms['windows-x86_64'].signature);
  assert.match(await readFile(join(root, 'release-assets/SHA256SUMS.txt'), 'utf8'), /^[a-f0-9]{64} \*Aywa_/);
  assert.deepEqual(JSON.parse(await readFile(join(root, 'release-assets/latest.json'), 'utf8')), metadata);
});

test('rejects inconsistent versions, extra installers, and invalid signatures', async (t) => {
  const { root, installerDir, name, tag } = await fixture(t);
  await assert.rejects(prepareUpdateAssets(root, { installerDir, tag: 'v99.0.0' }), /must match version/);
  await writeFile(join(installerDir, `${name}.sig`), 'invalid');
  await assert.rejects(prepareUpdateAssets(root, { installerDir, tag }), /malformed updater signature/);
  await writeFile(join(installerDir, 'extra.exe'), 'extra');
  await assert.rejects(prepareUpdateAssets(root, { installerDir, tag }), /Expected one installer/);
});

test('rejects a missing signature and an installer with the wrong architecture or version', async (t) => {
  const { root, installerDir, name, tag } = await fixture(t);
  await rm(join(installerDir, `${name}.sig`));
  await assert.rejects(prepareUpdateAssets(root, { installerDir, tag }), /ENOENT/);
  await rm(join(installerDir, name));
  await writeFile(join(installerDir, 'Aywa_0.0.0_arm64-setup.exe'), 'wrong installer');
  await assert.rejects(prepareUpdateAssets(root, { installerDir, tag }), /does not match/);
});
