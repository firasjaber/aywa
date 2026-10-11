import { access, readFile } from 'node:fs/promises';
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

async function exists(root, path) {
  try {
    await access(join(root, path));
    return true;
  } catch {
    return false;
  }
}

function cargoValue(cargoToml, key) {
  return new RegExp(`^${key} = "([^"]+)"$`, 'm').exec(cargoToml)?.[1];
}

export async function verifyReleasePolicy(root, { tag } = {}) {
  const failures = [];
  const [license, notice, thirdPartyNotices, packageJson, cargoToml, tauriJson, releaseChecklist] = await Promise.all([
    read(root, 'LICENSE'),
    read(root, 'NOTICE'),
    read(root, 'THIRD_PARTY_NOTICES.md'),
    read(root, 'package.json'),
    read(root, 'src-tauri/Cargo.toml'),
    read(root, 'src-tauri/tauri.conf.json'),
    read(root, 'docs/RELEASE_CHECKLIST.md').catch(() => '')
  ]);
  const packageMetadata = JSON.parse(packageJson);
  const tauriMetadata = JSON.parse(tauriJson);
  const cargoVersion = cargoValue(cargoToml, 'version');
  const cargoAuthors = /^authors = \["([^"]+)"\]$/m.exec(cargoToml)?.[1];

  if (!license.startsWith('MIT License')) {
    failures.push('LICENSE must contain the MIT License text.');
  }
  if (!notice.includes('Copyright (c) 2026 Firas Jaber')) {
    failures.push('NOTICE must identify Firas Jaber as the Aywa copyright owner.');
  }
  if (packageMetadata.license !== 'MIT') {
    failures.push('package.json must declare license "MIT".');
  }
  if (cargoValue(cargoToml, 'license') !== 'MIT') {
    failures.push('Cargo.toml must declare license "MIT".');
  }
  if (cargoValue(cargoToml, 'repository') !== 'https://github.com/firasjaber/aywa') {
    failures.push('Cargo.toml must declare the Aywa repository URL.');
  }
  if (cargoAuthors === undefined || cargoAuthors === 'you') {
    failures.push('Cargo.toml must identify a non-placeholder author.');
  }
  if (!cargoVersion || packageMetadata.version !== cargoVersion || tauriMetadata.version !== cargoVersion) {
    failures.push('package.json, Cargo.toml, and tauri.conf.json must use the same version.');
  }
  if (!Array.isArray(tauriMetadata.bundle?.targets) || tauriMetadata.bundle.targets.length !== 1 || tauriMetadata.bundle.targets[0] !== 'nsis') {
    failures.push('tauri.conf.json must target only the Windows NSIS bundle.');
  }
  const updater = tauriMetadata.plugins?.updater;
  if (tauriMetadata.bundle?.createUpdaterArtifacts !== true) {
    failures.push('tauri.conf.json must create updater artifacts.');
  }
  if (typeof updater?.pubkey !== 'string' || !Buffer.from(updater.pubkey, 'base64').toString('utf8').startsWith('untrusted comment: minisign public key:')) {
    failures.push('tauri.conf.json must contain the updater public key.');
  }
  if (JSON.stringify(updater?.endpoints) !== JSON.stringify(['https://github.com/firasjaber/aywa/releases/latest/download/latest.json']) || updater?.dangerousInsecureTransportProtocol || updater?.allowDowngrades) {
    failures.push('tauri.conf.json must use the stable HTTPS update endpoint without allowing insecure transport or downgrades.');
  }
  for (const path of ['SECURITY.md', 'docs/RELEASE_CHECKLIST.md']) {
    if (!(await exists(root, path))) {
      failures.push(`${path} must document public release operations.`);
    }
  }
  if (!releaseChecklist.includes('unsigned')) {
    failures.push('docs/RELEASE_CHECKLIST.md must disclose that installers are unsigned.');
  }
  if (!releaseChecklist.includes('SHA256SUMS.txt')) {
    failures.push('docs/RELEASE_CHECKLIST.md must document SHA256SUMS.txt verification.');
  }
  if (tag !== undefined) {
    if (!/^v\d+\.\d+\.\d+$/.test(tag)) {
      failures.push(`Release tag ${tag} must use the v<major>.<minor>.<patch> format.`);
    } else if (tag.slice(1) !== cargoVersion) {
      failures.push(`Release tag ${tag} must match version ${cargoVersion}.`);
    }
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
  const arguments_ = process.argv.slice(2);
  const tagIndex = arguments_.indexOf('--tag');
  const tag = tagIndex === -1 ? undefined : arguments_[tagIndex + 1];
  if (tagIndex !== -1 && (tag === undefined || tag.startsWith('--'))) {
    throw new Error('--tag requires a release tag such as v0.3.1.');
  }
  const rootArgument = arguments_.find((argument, index) => argument !== '--tag' && (tagIndex === -1 || index !== tagIndex + 1));
  const root = rootArgument ? resolve(rootArgument) : defaultRoot;
  const failures = await verifyReleasePolicy(root, { tag });
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
