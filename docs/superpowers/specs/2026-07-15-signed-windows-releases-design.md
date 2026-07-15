# Signed Windows Releases Design

## Purpose

Make Aywa's public Windows releases reproducible, verifiable, and consistently
identified as version `0.1.0` by adding Windows-only continuous integration,
a fail-closed signed release pipeline, supporting policy documentation, and
metadata validation.

## Scope

Aywa supports Windows only. Pull requests and pushes validate the application
on GitHub-hosted Windows runners. A pushed Git tag matching `v*` starts a
release job that validates the tag and project metadata, builds the Windows
installer, signs it with certificate material stored only in GitHub Actions
secrets, verifies the signature, runs an installation smoke test on a fresh
runner, produces SHA-256 checksum files, and publishes the installer and
checksums to a GitHub Release.

The workflow must fail before publication if signing credentials are absent or
the signature cannot be verified. It must never fall back to an unsigned
public release.

## Version ownership

`src-tauri/tauri.conf.json` remains the canonical user-facing application
version. Cargo and the frontend package manifest contain the same checked-in
version so their normal tooling reports `0.1.0`. A verification script checks
all three values and requires a release tag such as `v0.1.0` to match them.
Release automation does not rewrite committed manifests.

## Components

- `.github/workflows/ci.yml` runs on `windows-2022` for pull requests and
  main-branch pushes. It installs the checked-in Node dependency graph and a
  declared Rust toolchain, then runs frontend, Rust, Python, and release-policy
  checks.
- `.github/workflows/release.yml` runs only for pushed `v*` tags. It performs
  all CI checks, validates the tag/version, builds the Windows NSIS installer,
  imports a base64-encoded PFX certificate from GitHub Actions secrets, signs
  the installer with SignTool, verifies the Authenticode signature, hashes the
  public artifacts, smoke-installs the installer silently, and creates the
  GitHub Release.
- `rust-toolchain.toml` pins the Rust compiler used by CI and contributors.
- A small Node release-policy verifier/test suite validates metadata, Windows
  target scope, security/release documentation, and synchronized versions.
- `SECURITY.md` supplies a private vulnerability-reporting path and disclosure
  expectations. `docs/RELEASE_CHECKLIST.md` explains secret setup, preflight,
  tag creation, artifact verification, smoke testing, and rollback/revocation.

## Signing credentials

Repository administrators configure these Actions secrets before the first
tagged release:

- `WINDOWS_CERTIFICATE_PFX_BASE64`: base64 representation of the code-signing
  certificate PFX file.
- `WINDOWS_CERTIFICATE_PASSWORD`: password that protects that PFX file.

The certificate is decoded into the runner's temporary directory only for the
signing step. Its path is not emitted, and the file is deleted regardless of
whether signing succeeds. No certificate is committed to the repository.

## Artifact flow

```text
push v0.1.0
  -> validate manifests and tag
  -> test and build on Windows
  -> sign + verify NSIS installer
  -> compute SHA-256SUMS.txt
  -> silent installer smoke test
  -> GitHub Release with installer + checksum file
```

The release is created only after every preceding step succeeds. Users can
verify a downloaded installer with `Get-FileHash -Algorithm SHA256` and the
published checksum file.

## Error handling and security

Workflow steps use PowerShell with terminating-error behavior. The signing
job checks that both required secrets exist before certificate decoding and
uses GitHub secret masking. Signature verification, checksum generation, and
the install smoke test are independent release gates. The smoke test confirms
that a silent install creates the expected installed executable and then
uninstalls it. A failed test prevents release publication.

## Validation

- Run the release-policy script and its Node tests locally.
- Run `npm run check`, `npm run build`, Rust tests, and Python tests.
- Validate workflow YAML structure and inspect the final diff.
- In GitHub, run the workflow from a `v0.1.0` tag only after configuring
  signing secrets. Confirm that the published installer is signed, its hash
  matches `SHA256SUMS.txt`, and the release smoke test completed.

## Out of scope

- Purchasing or issuing a code-signing certificate.
- Publishing `v0.1.0` or creating the tag on the maintainer's behalf.
- macOS, Linux, updater, notarization, or Microsoft Store distribution.
