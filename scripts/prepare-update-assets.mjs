import { copyFile, mkdir, readFile, readdir, writeFile } from 'node:fs/promises';
import { createHash } from 'node:crypto';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { verifyReleasePolicy } from './verify-release-policy.mjs';

export async function prepareUpdateAssets(root, { tag, installerDir = join(root, 'src-tauri/target/release/bundle/nsis'), outputDir = join(root, 'release-assets') }) {
  const failures = await verifyReleasePolicy(root, { tag });
  if (failures.length) throw new Error(failures.join('\n'));
  const version = tag.slice(1);
  const installers = (await readdir(installerDir)).filter((name) => name.endsWith('.exe'));
  if (installers.length !== 1) throw new Error(`Expected one installer, found ${installers.length}.`);
  const name = installers[0];
  if (name !== `Aywa_${version}_x64-setup.exe`) throw new Error('Installer filename does not match the release version and Windows architecture.');
  const signature = (await readFile(join(installerDir, `${name}.sig`), 'utf8')).trim();
  const decodedSignature = Buffer.from(signature, 'base64').toString('utf8');
  if (!decodedSignature.startsWith('untrusted comment:') || !decodedSignature.includes('\ntrusted comment:') || decodedSignature.trim().split('\n').length !== 4) {
    throw new Error('Missing or malformed updater signature.');
  }
  const bytes = await readFile(join(installerDir, name));
  if (!bytes.length) throw new Error('Installer is empty.');
  const metadata = {
    version,
    pub_date: new Date().toISOString(),
    platforms: {
      'windows-x86_64': {
        url: `https://github.com/firasjaber/aywa/releases/download/${tag}/${encodeURIComponent(name)}`,
        signature
      }
    }
  };
  await mkdir(outputDir, { recursive: true });
  const expected = [name, `${name}.sig`, 'latest.json', 'SHA256SUMS.txt'];
  if ((await readdir(outputDir)).some((file) => !expected.includes(file))) throw new Error('Release directory contains unexpected assets.');
  await copyFile(join(installerDir, name), join(outputDir, name));
  await writeFile(join(outputDir, `${name}.sig`), signature);
  await writeFile(join(outputDir, 'latest.json'), `${JSON.stringify(metadata, null, 2)}\n`);
  await writeFile(join(outputDir, 'SHA256SUMS.txt'), `${createHash('sha256').update(bytes).digest('hex')} *${name}\n`);
  return metadata;
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  const root = resolve(dirname(fileURLToPath(import.meta.url)), '..');
  const tag = process.argv[2];
  if (!tag) throw new Error('Usage: node scripts/prepare-update-assets.mjs v<major>.<minor>.<patch>');
  await prepareUpdateAssets(root, { tag });
  console.log('Installer, update signature, metadata, and checksum are ready.');
}
