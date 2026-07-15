import { readFile } from 'node:fs/promises';
import { fileURLToPath } from 'node:url';
import { dirname, join, resolve } from 'node:path';

const requiredNoticePhrases = [
  'CPython embeddable distribution',
  'get-pip.py',
  'python.org',
  'bootstrap.pypa.io',
  'download.pytorch.org',
  'huggingface.co',
  'SBOM'
];

function read(root, path) {
  return readFile(join(root, path), 'utf8');
}

export async function verifyReleasePolicy(root) {
  const failures = [];
  const [license, notice, thirdPartyNotices, packageJson, cargoToml] = await Promise.all([
    read(root, 'LICENSE'),
    read(root, 'NOTICE'),
    read(root, 'THIRD_PARTY_NOTICES.md'),
    read(root, 'package.json'),
    read(root, 'src-tauri/Cargo.toml')
  ]);
  const packageMetadata = JSON.parse(packageJson);

  if (!license.startsWith('MIT License')) {
    failures.push('LICENSE must contain the MIT License text.');
  }
  if (!notice.includes('Copyright (c) 2026 Firas Jaber')) {
    failures.push('NOTICE must identify Firas Jaber as the Aywa copyright owner.');
  }
  if (packageMetadata.license !== 'MIT') {
    failures.push('package.json must declare license "MIT".');
  }
  if (!/^license = "MIT"$/m.test(cargoToml)) {
    failures.push('Cargo.toml must declare license "MIT".');
  }
  if (!/^repository = "https:\/\/github\.com\/firasjaber\/aywa"$/m.test(cargoToml)) {
    failures.push('Cargo.toml must declare the Aywa repository URL.');
  }
  for (const phrase of requiredNoticePhrases) {
    if (!thirdPartyNotices.includes(phrase)) {
      failures.push(`THIRD_PARTY_NOTICES.md must mention "${phrase}".`);
    }
  }
  return failures;
}

async function main() {
  const defaultRoot = resolve(dirname(fileURLToPath(import.meta.url)), '..');
  const root = process.argv[2] ? resolve(process.argv[2]) : defaultRoot;
  const failures = await verifyReleasePolicy(root);
  for (const failure of failures) {
    console.error(`release-policy: ${failure}`);
  }
  if (failures.length > 0) {
    process.exitCode = 1;
  }
}

if (process.argv[1] === fileURLToPath(import.meta.url)) {
  main();
}
