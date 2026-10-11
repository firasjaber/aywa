# Unsigned Windows Releases Design

## Purpose

Allow Aywa to publish `v*` Windows releases without code-signing credentials
while retaining reproducible validation, SHA-256 download checksums, and a
clean-runner installer smoke test.

## Release flow

A pushed tag such as `v0.1.0` triggers a Windows-only GitHub Actions workflow.
The workflow validates that the tag matches the synchronized project version,
runs the frontend, Rust, Python, and release-policy checks, prepares bundled
Python resources, builds the NSIS installer, computes `SHA256SUMS.txt`, silently
installs the installer on the fresh GitHub-hosted runner, and creates a GitHub
Release containing both files.

The workflow does not read certificate secrets, import PFX files, invoke
SignTool, or claim that the installer is signed. A failed check, build, hash,
or smoke install prevents release publication.

## User-facing trust model

Release documentation and user guides explicitly describe installers as
unsigned. Windows may show an unknown-publisher or SmartScreen warning. Users
must download only from Aywa's GitHub Releases page and compare the installer
SHA-256 value from PowerShell with the corresponding `SHA256SUMS.txt` line.

Checksums detect corruption or a changed file after the release is published;
they do not provide publisher identity or replace code signing. The documents
state this boundary plainly.

## Scope

Keep Windows-only CI, NSIS-only Tauri packaging, version/tag enforcement, the
security-vulnerability reporting policy, and the compile-time Windows guard.
Remove only signing-specific workflow code, secret names, certificate setup
instructions, signature verification directions, and claims of Authenticode or
publisher verification.

## Validation

- Policy tests and release-tag validation pass with no signing secrets.
- Static workflow review confirms no PFX, SignTool, certificate, or signature
  references remain in active release automation.
- Frontend, Rust, and Python checks pass unchanged.
- Markdown search confirms README, setup guide, user guide, and release
  checklist accurately describe unsigned installers and checksums.
