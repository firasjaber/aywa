# Windows release checklist

Aywa releases support 64-bit Windows only. A pushed version tag starts the
Windows release workflow. Installers are currently unsigned.

## Before creating a release

- Confirm the tracked checkout is clean and `package.json`, `src-tauri/Cargo.toml`,
  and `src-tauri/tauri.conf.json` all say the intended version.
- Run `npm.cmd run verify:release-policy`, `npm.cmd run check`, `npm.cmd run build`,
  `cargo test --manifest-path src-tauri/Cargo.toml --locked`, and
  `python -m pytest src-tauri/python/tests -v`.
- GitHub does not run checks for ordinary branch pushes or pull requests. Run
  the local checks before tagging; the heavyweight GitHub validation runs only
  after the release tag is pushed.
- Confirm release notes describe user-visible changes and known limitations.
- Remember that the installer will be unsigned; do not describe it as signed
  or claim a verified publisher identity.

## Create `0.3.0`

After pushing the release commit and confirming Windows CI is green, create and
push the immutable annotated tag:

```powershell
git status
git tag -a v0.3.0 -m "Aywa 0.3.0"
git push origin v0.3.0
```

The `Unsigned Windows Release` workflow checks the tag and manifest versions,
runs tests, prepares locked Python resources, builds the NSIS installer, writes
`SHA256SUMS.txt`, silently installs the installer on a fresh Windows runner,
and only then creates the GitHub Release.

Public tags are permanent release identifiers. If a published tag is wrong, do
not force-move it; fix the release and publish a new patch version such as
`v0.3.1`.

## Verify the published release

Download the installer and `SHA256SUMS.txt` from the matching Aywa GitHub
Release. Verify the downloaded installer locally:

```powershell
Get-FileHash .\Aywa_0.3.0_x64-setup.exe -Algorithm SHA256
```

Compare the lowercase SHA-256 output with the installer line in
`SHA256SUMS.txt`. This confirms the downloaded file matches the file Aywa
published in that release; it does not identify the publisher or replace code
signing. Only download installers from Aywa's GitHub Releases page. Windows may
show an unknown-publisher or SmartScreen warning because the installer is
unsigned.

## Failed release

If a workflow fails before GitHub Release publication, fix the reported cause
and push a new version tag. Do not bypass the version check, test suite,
checksum generation, or smoke-install gate.
