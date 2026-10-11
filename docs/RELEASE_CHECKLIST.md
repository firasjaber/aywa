# Windows release checklist

Aywa releases support 64-bit Windows only. A pushed version tag starts the
`Windows Release` workflow. Update downloads use a signing key trusted by Aywa;
installers remain unsigned for Windows publisher verification (Authenticode).

## Configure signing before the first update release

The matching private key is stored at
`%LOCALAPPDATA%\AywaRelease\updater.key` on the maintainer's machine. The public
key is in `src-tauri/tauri.conf.json`. The private key is restricted to the
Windows user who created it. Keep an encrypted backup in a secure location
outside the repository and outside release artifacts. Losing it prevents
existing installations from accepting future signed updates.

Create the GitHub Actions repository secret `TAURI_SIGNING_PRIVATE_KEY` from
the contents of that file. The workflow cannot use a path on your computer.
The generated key has an empty password. If you encrypt it, also configure
`TAURI_SIGNING_PRIVATE_KEY_PASSWORD`. Never print a private key, commit it, or
upload it as an artifact. Do not generate a replacement key for each release.

See [local signing setup](SETUP.md#build-a-windows-installer) to build an installer
without exposing the key in command output.

## Before creating a release

- Confirm the tracked checkout is clean and `package.json`, `package-lock.json`,
  `src-tauri/Cargo.toml`, the Aywa entry in `src-tauri/Cargo.lock`, and
  `src-tauri/tauri.conf.json` all say the intended version.
- Run `npm.cmd run verify:release-policy -- --tag v0.3.1`, `npm.cmd run check`, `npm.cmd run test`,
  `npm.cmd run build`,
  `node --test scripts/prepare-update-assets.test.mjs scripts/verify-release-policy.test.mjs`,
  `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check`,
  `cargo test --manifest-path src-tauri/Cargo.toml --locked`, and
  `python -m pytest src-tauri/python/tests -v`.
- Confirm the signing secret matches the public key configured in the app.
- GitHub validation runs after a release tag is pushed. Run the local checks
  before tagging; ordinary branch pushes and pull requests do not run this workflow.
- Describe user-visible changes and known limitations in the release notes.
  Explain that users on a version without update controls need one manual
  installer update. They should install over their existing version.
- Do not claim a Windows verified publisher identity. An update signature
  does not replace an Authenticode certificate.

## Create a release

After pushing the release commit, create and push the immutable annotated tag
for that version. For version `0.3.1`:

```powershell
git status
git tag -a v0.3.1 -m "Aywa 0.3.1"
git push origin v0.3.1
```

The workflow checks the tag and manifest versions, runs tests, prepares locked
Python resources, builds the NSIS installer, and signs the update. It validates
the actual installer signature using the app's public key, including rejection
of altered installer bytes. It then tests a fresh installation.

For the upgrade check, it builds an unpublished `0.0.0` baseline with update
artifacts disabled. On the isolated GitHub-hosted Windows runner,
`scripts/smoke-test-update.ps1` installs that baseline, seeds settings, rules,
History, and model/runtime sentinel files, then upgrades with `/S /UPDATE /R`.
It verifies the executable version, app restart, and unchanged seeded data.
This checks the NSIS upgrade path; it does not simulate clicking the footer.
Do not run this installer test on a user's machine or reuse an active data folder.

The workflow creates a draft with exactly four assets: the `.exe`, its `.exe.sig`,
`latest.json`, and `SHA256SUMS.txt`. It verifies the names, sizes, and downloaded
hashes before publishing. The app reads
`https://github.com/firasjaber/aywa/releases/latest/download/latest.json`.
The manifest contains a stable version and a `windows-x86_64` entry with the
release-specific HTTPS installer URL and its complete signature.

Public tags are permanent release identifiers. If a published tag is wrong,
fix the release and publish a new patch version such as `v0.3.2`.

## Verify the published release

Confirm all four assets are present and that `latest.json` refers to the
installer from the same release. Download the installer and `SHA256SUMS.txt`
from the matching Aywa GitHub Release, then verify the installer:

```powershell
Get-FileHash .\Aywa_0.3.1_x64-setup.exe -Algorithm SHA256
```

Compare the lowercase SHA-256 output with the installer line in
`SHA256SUMS.txt`. A checksum confirms file integrity; it does not identify the
publisher. Only download installers from Aywa's GitHub Releases page. Windows
may show an unknown-publisher or SmartScreen warning.

On a machine with a previous updater-enabled release, select **Check for
updates** and confirm the newer version appears. Finish active dictation or
model work, then select **Download and install**. Verify restart and preserved
local data. Record this interactive check separately from the CI smoke tests.

## Failed release or update

If the workflow fails, fix the reported cause and publish a new version tag.
Inspect and remove any unpublished draft with incomplete assets before retrying.
Keep the version, signature, asset, checksum, and Windows smoke-test gates enabled.

If an installed app cannot update, retry in its footer or install the latest
GitHub release over the existing installation. Keep the local app data folder.
If the private key is lost, restore the matching backup; changing the public
key alone will not let old installations accept the replacement key.
