# ADR 0002: Use signed in-app updates

Status: Accepted
Date: 2026-10-11

## Context

Users need to update Aywa without removing and reinstalling it. Aywa uses Tauri 2 and distributes Windows NSIS installers through GitHub Releases. Updates must preserve local data and avoid interrupting speech or model work.

## Decision

Use Tauri's updater through Rust commands owned by the app. Check the fixed GitHub latest-release feed at startup and once a day. Offer only newer stable versions for Windows x64. The shared footer provides manual checks and user-controlled installation.

Keep updater download URLs and signatures in Rust. Do not expose the plugin's installation commands directly to the frontend. Verify signatures before installer execution.

Use one admission guard for recording, transcription, model work, runtime work, and installer handoff. Work permits last until each operation finishes. An exclusive installation permit blocks new conflicting work. If work starts during a download, retain the verified bytes for a later installation attempt.

Keep the public key in app configuration. Store the matching private key outside Git, with restricted access and a secure backup. GitHub Actions uses repository signing secrets. Publish the installer, signature, manifest, and checksum as a complete draft before exposing the release through the latest-release feed.

Keep the app identifier and local data paths unchanged. The installer replaces the application and restarts it. It must preserve settings, rules, History, models, and runtime files.

## Consequences

GitHub hosts update assets without a separate server. Users choose when to download and install. Rust enforces installation safety even if the displayed status is stale.

Release maintenance requires the matching private key and signing secrets. Losing the key requires recovery from its backup. Replacing the public key alone cannot update old installations that trust the previous key.

Update signatures do not provide Windows Authenticode publisher verification. Users on older versions need one manual installer update to receive these controls. Recovery uses the GitHub installer if installation fails after Aywa exits. Automatic rollback is outside this feature.

## Alternatives

A footer link to GitHub would reduce implementation work, but users would still manage installer downloads themselves. The user selected in-app installation. A separate update service would add infrastructure that the public GitHub release feed already provides.

## References

- [Approved spec](../superpowers/specs/2026-10-11-in-app-updates-design.md)
- [Implementation plan and review](../implementation-plans/2026-10-11-in-app-updates.md)
- [Rust updater](../../src-tauri/src/updater.rs)
- [Release workflow](../../.github/workflows/release.yml)
- [Signing setup and release checks](../RELEASE_CHECKLIST.md)
