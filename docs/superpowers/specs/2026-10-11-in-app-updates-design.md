# In-app updates for Aywa

## Goal and context

Users can discover and install new Aywa releases from the app without manually removing the current installation.
Aywa uses Tauri 2 and publishes a Windows NSIS installer through GitHub Releases when a version tag is pushed.
The existing release workflow tests a fresh installation but does not test an upgrade.

The user selected the in-app updater instead of a footer link that only opens GitHub.
The timing, footer controls, and recovery behavior below are proposed defaults for review.

## Discovery and controls

Aywa checks for an update after launch without delaying startup or opening the Settings window.
It checks again every 24 hours while the app remains open, including while Settings is hidden.
Only a published stable release with a higher version and a matching Windows installer qualifies as an update.
Equal or older versions must not appear as updates.

The shared Settings footer shows the installed version and a `Check for updates` control on every tab.
If an update exists, it shows the new version and a `Download and install` button.
The footer explains that installation closes and restarts Aywa.
It retains the existing author and source links and remains usable in a small window.
Update discovery does not trigger a popup or an automatic download.

An automatic check failure stays quiet and does not claim that the app is up to date.
A manual check shows its result or a short error with a retry control.
Concurrent checks and repeated clicks must not start duplicate operations.

## Download and installation

The user starts the update with `Download and install`.
Aywa shows real download progress, using a busy indicator when the download size is unavailable.
Aywa installs only an update whose signature passes verification.
A download or verification failure leaves the current installation usable and offers a retry.

The action is unavailable during recording, transcription, model downloads, model loading, or runtime preparation and testing.
The footer explains that the user must finish that work first.
The app enforces this rule when the action starts and immediately before installation.
When installation begins, new speech and model operations must not start.
Aywa stops its transcription worker cleanly before the Windows installer takes over.
If preparation for installation fails, Aywa stays open, shows an error, and allows normal use to resume.

The installer replaces the installed app and launches the new version after a successful update.
The user does not need to uninstall Aywa manually.
Updating must preserve settings, transcription rules, saved history, downloaded models, and local runtime files.
Future runtime compatibility changes can still require runtime preparation, as documented today.

If the installer fails after Aywa exits, the app cannot display an error in its closed window.
The user guide explains how to run the installer from GitHub Releases to recover.
This feature does not promise automatic rollback after an installer failure.

## Release requirements

Use Tauri's supported updater and the existing public GitHub Releases repository.
GitHub hosts the installer, its update signature, and the metadata that describes the latest stable update.
No separate update server is required.
Public updates continue to use the existing synchronized version and version-tag rules.

An update signature proves that the downloaded installer matches the trusted signing key.
The app contains the public key, and release automation uses the private key from GitHub Actions secrets.
The private key stays outside the repository and requires a separate secure backup.
Release publication fails if the required signing credentials or update assets are missing.
Publish a release only after its complete installer, signature, metadata, and checksums pass validation.

Update signing does not replace Windows certificate signing.
Aywa's Windows installers remain unsigned under the current release policy.
The documentation must distinguish these two uses of signing.
No installer or signing secret is published as part of preparing this feature.
Users on older releases need one manual installation to receive the updater.

## Success criteria

- A newer stable release appears automatically and through a manual check.
- Equal, older, and prerelease versions do not produce an update offer.
- An offline or invalid response leaves Aywa usable, with an accurate manual-check result.
- The update controls show download progress and prevent duplicate actions.
- Busy work prevents installation, and installation prevents new conflicting work.
- An invalid signature prevents installer execution.
- A real Windows upgrade starts the new version and preserves representative local data.
- Release automation produces complete signed update assets and retains existing release checks.

## Source

[Tauri updater documentation](https://v2.tauri.app/plugin/updater/) describes supported GitHub hosting, mandatory update signatures, and the Windows exit behavior.
