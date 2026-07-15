# Signed Windows release checklist

Aywa releases support 64-bit Windows only. A pushed version tag starts the
Windows release workflow; it never publishes an unsigned installer.

## One-time repository setup

Obtain a trusted Windows code-signing certificate in password-protected PFX
format. Do not commit the PFX file or its password. In GitHub repository
**Settings → Secrets and variables → Actions**, add:

- `WINDOWS_CERTIFICATE_PFX_BASE64`: the PFX file encoded as one base64 string.
- `WINDOWS_CERTIFICATE_PASSWORD`: the PFX password.

To create the first value locally in PowerShell, replacing the path with the
certificate location:

```powershell
[Convert]::ToBase64String([IO.File]::ReadAllBytes('C:\secure\aywa-signing.pfx'))
```

Copy the output directly into the GitHub secret, then remove it from any shell
history or temporary notes. The release workflow writes the certificate only to
the ephemeral runner, signs the installer, verifies it, and deletes the file.

## Before creating a release

- Confirm the tracked checkout is clean and `package.json`, `src-tauri/Cargo.toml`,
  and `src-tauri/tauri.conf.json` all say the intended version.
- Run `npm run verify:release-policy`, `npm run check`, `npm run build`,
  `cargo test --manifest-path src-tauri/Cargo.toml --locked`, and
  `python -m pytest src-tauri/python/tests -v`.
- Confirm the signing secrets are present and valid without revealing their
  values in issues, logs, or commits.
- Confirm any release notes explain user-visible changes and known limitations.

## Create `0.1.0`

After pushing the release commit and confirming Windows CI is green, create and
push the immutable annotated tag:

```powershell
git status
git tag -a v0.1.0 -m "Aywa 0.1.0"
git push origin v0.1.0
```

The `Signed Windows Release` workflow runs from that tag. It checks the tag and
all manifest versions, runs tests, prepares locked Python resources, builds the
NSIS installer, signs and verifies it, writes `SHA256SUMS.txt`, silently installs
the installer on the fresh runner, and only then creates the GitHub Release.

Public tags are permanent release identifiers. If a published tag is wrong, do
not force-move it; fix the release and publish a new patch version such as
`v0.1.1`.

## Verify the published release

Download the installer and `SHA256SUMS.txt` from the matching GitHub Release.
Verify its hash locally:

```powershell
Get-FileHash .\Aywa_0.1.0_x64-setup.exe -Algorithm SHA256
Get-AuthenticodeSignature .\Aywa_0.1.0_x64-setup.exe | Format-List Status,SignerCertificate
```

Compare the lowercase SHA-256 output with the installer line in
`SHA256SUMS.txt`; `Status` should be `Valid` and the signer should be the
expected publisher. Retain the workflow artifact and release assets as the
record of what was tested and published.

## Certificate incident or failed release

If the certificate is lost, exposed, or suspected compromised, revoke it with
the issuing certificate authority, remove or rotate both Actions secrets, and
publish a newly signed replacement release. If a workflow fails before the
GitHub Release step, fix the cause and push a new version tag; do not bypass the
signing, verification, checksum, or smoke-install gates.
