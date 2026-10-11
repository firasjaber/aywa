# In-app updates implementation plan

[Approved spec](../superpowers/specs/2026-10-11-in-app-updates-design.md)

## Goal and approach

Add automatic update discovery and user-controlled installation through Tauri's updater and GitHub Releases.
Use the current checkout and retain the existing Windows NSIS packaging and unsigned Windows certificate policy.
Do not publish a release, push a tag, or upload a private signing key during implementation.

The Rust app owns the updater, its schedule, and its installation guard.
The Svelte interface uses the existing `invoke` and `listen` patterns for status and actions.
Use the updater plugin's Rust API without JavaScript updater or process plugins.
Use the existing `transcribe::unload` function before installer execution.
Keep `com.aywa.speech` and all local data paths unchanged.

## Interfaces and important decisions

Add `src-tauri/src/updater.rs` for the updater state, commands, schedule, and operation guard.
Register the state and plugin in `src-tauri/src/main.rs`, and export the module through `src-tauri/src/lib.rs`.
Use the following command names:

- `get_update_status`: Return the current snapshot without performing a network request.
- `check_for_updates`: Perform a manual check and return its final snapshot.
- `install_update`: Start one download and installation attempt for the stored update.

Emit `app-update-changed` with the same snapshot returned by the commands.
Serialize field names in camelCase, following the other app snapshots.
The snapshot contains `revision`, `currentVersion`, `availableVersion`, `status`, `downloadedBytes`, `totalBytes`, `downloadReady`, and `error`.
Increase `revision` on each state change so delayed command responses cannot replace newer event state.
`downloadReady` is true only while verified installer bytes remain cached for the stored update.
Status values are `idle`, `checking`, `upToDate`, `available`, `downloading`, `installing`, and `error`.
`availableVersion`, `totalBytes`, and `error` can be null.
Keep the plugin's update object in Rust instead of accepting a download URL or signature from the interface.

Use `https://github.com/firasjaber/aywa/releases/latest/download/latest.json` as the fixed production endpoint.
Use a 15-second timeout for metadata checks and a five-minute timeout for installer downloads.
Reject prerelease versions as well as equal and older versions.
Use the plugin's version comparison and reject a hyphen before the optional `+` build-metadata suffix in its normalized version.
Do not add a separate version parser library for this restriction.

Store the private key outside the repository at `%LOCALAPPDATA%\AywaRelease\updater.key`.
Generate it during implementation only if no existing Aywa update key is configured.
Never overwrite an existing key, print its contents, or add it to Git.
Restrict the private-key file to the Windows user who creates it.
If an existing public key is configured but its private key is unavailable, report the missing key instead of replacing it.
Commit only the matching public key in Tauri configuration.
The release maintainer must back up the private key and configure GitHub Actions secrets before the first updater-enabled release.
No missing-key placeholder can remain in the implementation.

## Tasks

### [x] Task 1: Configure the supported updater and signing key

Modify `src-tauri/Cargo.toml`, `src-tauri/Cargo.lock`, `src-tauri/tauri.conf.json`, and `src-tauri/src/main.rs`.
Add the compatible `tauri-plugin-updater` 2.x Rust dependency and lock its resolved version.
Enable Tauri's `test` feature only in Cargo dev-dependencies for the mock application used by updater tests.
Initialize the updater plugin, set `bundle.createUpdaterArtifacts` to `true`, and configure the public key and fixed endpoint.
Use the Windows `passive` installation mode and the plugin's default restart behavior.
Do not grant frontend access to plugin installation commands, since all installation requests go through guarded Rust commands.

Use the existing Tauri CLI signer to create the key outside the repository.
Read the CLI help before generating the key, and suppress any private-key output.
Sign a small fixture for Task 2 and retain only its harmless payload, public key, and signature in the test data.
Keep fixture signing isolated from public release assets, and do not require private keys to run the tests.

Run `cargo check --manifest-path src-tauri/Cargo.toml --locked`.
Confirm the updater uses HTTPS and retains signature verification.

### [x] Task 2: Implement automatic discovery and accurate status

Create `src-tauri/src/updater.rs` and export it from `src-tauri/src/lib.rs`.
Register updater state and the three commands in `src-tauri/src/main.rs`.
Start one Rust async task after app setup for an immediate check and subsequent checks every 86,400 seconds.
The task remains active while Settings is hidden and does not reveal any window.
Skip automatic checks in development builds to avoid offering production installations over development checkouts.
Manual checks remain available for controlled development verification.

Use a short mutex lock to claim each check or install operation and release the lock before network work.
Duplicate manual requests return the current status instead of starting another request.
Use Tauri's updater API to fetch and retain a qualifying update for `windows-x86_64`.
An automatic failure preserves any known update and produces no user-facing error.
It must not change an unknown result into `upToDate`.
A manual failure sets a short actionable error while retaining a previously known update.
Treat a missing feed, invalid JSON, missing target, and timeout as failures rather than successful no-update checks.
Log failures locally without emitting credentials or response bodies.

Add Rust tests in the module using a loopback HTTP fixture and the plugin's real check and download APIs.
Use test-only endpoint and transport overrides without exposing them as production commands.
Cover newer, equal, older, and prerelease versions, malformed or missing metadata, and automatic versus manual failure behavior.
Prove duplicate checks cannot overlap and signature failure prevents an installation attempt.
Tests must never execute an installer on the developer's machine.
Use the signed fixture from Task 1 to prove a valid signature succeeds and changed bytes fail with the configured public key.

Run `cargo test --manifest-path src-tauri/Cargo.toml --locked updater`.

### [x] Task 3: Guard conflicting work and perform installation

Modify `src-tauri/src/updater.rs`, `src-tauri/src/lib.rs`, and `src-tauri/src/model_manager/service.rs`.
Add one operation guard with an active-work count and an exclusive installation flag under the same mutex.
An operation permit releases its count automatically when dropped.
The installation permit succeeds only when the work count is zero, and it blocks new work until dropped or the process exits.
Never hold the mutex across network requests, worker shutdown, or disk operations.

Acquire work permits before recording starts, before model downloads and loads are scheduled, and before runtime work begins.
Store the recording permit in `RuntimeHandles` until `stop_recording` finishes transcription, history, and clipboard output.
Release that permit on recording-start failure and every stop or transcription failure path.
Move permits into spawned model and runtime threads so they last until the job actually finishes.
Guard runtime selection, profile application, testing, redetection, model unloading, and cache clearing before their first mutation.
Retain permits across their nested bootstrap scheduling to close the gap between changing configuration and starting work.
Do not count normal settings reads or updater metadata checks as conflicting work.

At `install_update`, reject current conflicting work before download and prevent duplicate update attempts.
Use `Update::download` for progress and signature verification before installer handoff.
Allow normal use during download, then acquire the exclusive installation permit after download.
If work started during download, retain the verified download and offer installation again when idle.
Set `downloadReady` for this retry and clear it when a check discovers a different update.
Show an accurate busy explanation instead of discarding or executing the installer.
Recheck app and model state under the same admission boundary before accepting installation.

Run `transcribe::unload` only after acquiring the exclusive permit.
Keep recording and model entry points blocked through shutdown and the call to `Update::install`.
Tauri launches the NSIS installer, exits Aywa, and requests restart after installation.
If shutdown or installer launch fails while Aywa remains open, release the permit and restore normal commands.
Reconcile loaded-model state after a worker shutdown followed by installer-launch failure so the interface allows the user to load the model again.
Do not delete settings, history, model, or runtime files.

Add focused Rust tests for work-versus-install admission, permit release on failure, asynchronous permit lifetime, and cached-download retry after busy work.
Include a concurrent admission test proving an installer and a new work permit cannot both succeed.
Check every caller of the guarded entry points to prevent an unguarded hotkey or startup path.
Run the full Rust suite with `cargo test --manifest-path src-tauri/Cargo.toml --locked`.

### [x] Task 4: Add footer controls and download progress

Create `src/lib/stores/appUpdate.ts` and `src/lib/components/AppUpdate.svelte`.
Modify `src/lib/components/Settings.svelte` to render the update component within the shared footer.
Follow the existing `modelManager` store pattern: subscribe to events before loading a snapshot, then clean up listeners.
Ignore a snapshot with an older `revision`, including a delayed initial command response.
Use the command and snapshot names defined above.
Use the existing app and model stores to explain busy work and disable the update action.
Rust remains responsible for rejecting races even if the displayed status is stale.

Show the installed version and `Check for updates` on every tab.
Show a newer version, `Download and install`, and the close-and-restart explanation when an update exists.
Render real byte progress, a busy indicator for unknown size, and distinct checking and installation states.
If a verified download waits for idle work, show `Install update` without implying another download is needed.
Use short errors with a retry action and do not replace the existing author and source links.
Use native buttons, accessible status announcements, and a wrapping layout for the minimum window size.

Add `src/lib/stores/appUpdate.test.ts` with mocked Tauri commands and events.
Cover an update arriving during initialization, event cleanup, unknown-size progress, and retry after a failed request.
Use the actual store behavior rather than adding helpers solely for tests.
Manually inspect the footer on every tab at the smallest supported window size and verify keyboard access.
Run `npm.cmd run test`, `npm.cmd run check`, and `npm.cmd run build`.

### [x] Task 5: Publish complete update assets and test Windows upgrades

Modify `.github/workflows/release.yml` and `scripts/verify-release-policy.mjs` with its existing test file.
Add `scripts/prepare-update-assets.mjs` and `scripts/prepare-update-assets.test.mjs`.
Keep Node's standard library and the existing test runner for asset generation and validation.

Require `TAURI_SIGNING_PRIVATE_KEY` before building a release.
Pass it and the optional `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` only to the signing build step.
Retain existing tag/version validation, frontend checks, Rust and Python tests, checksums, and the fresh-install test.
Include the frontend and Node script tests in the release checks.
Fail explicitly on a nonzero exit code from each native command in the PowerShell workflow.

Generate `latest.json` from the validated project version, actual NSIS filename, and generated signature file.
Its `windows-x86_64` entry contains the release-specific HTTPS installer URL and the complete signature contents.
Publish the `.exe`, `.exe.sig`, `latest.json`, and `SHA256SUMS.txt` together.
Create the release as a draft, upload and verify all expected assets, and only then mark it published.
This avoids exposing an incomplete release through the latest-release endpoint.
Reject missing or empty signatures, inconsistent versions, multiple installers, and malformed metadata before publication.
Add Node tests for the valid asset set and these realistic failure cases.
Extend release-policy tests to require the updater public key, HTTPS endpoint, and artifact generation without requiring a Windows certificate.

Add `scripts/smoke-test-update.ps1` for use on the isolated GitHub Windows runner.
Build an unpublished baseline NSIS installer from the same source with a temporary Tauri version override of `0.0.0`.
Keep its updater artifact generation disabled and store its output separately from the public release assets.
Install the baseline, then seed valid settings with rules, saved-history fixtures, and sentinel files under model and runtime data paths.
Install the signed target installer with `/S /UPDATE /R`, then assert its executable version, launch, and preserved data.
The script must check installer exit codes and clean up only the processes and paths it created on the runner.
Keep automatic key backup outside this script and outside GitHub artifacts.
Do not run this destructive installer smoke test against the user's active Windows installation.

Run `node --test scripts/prepare-update-assets.test.mjs scripts/verify-release-policy.test.mjs` and `npm.cmd run verify:release-policy`.
Make the Task 2 download test accept an optional `AYWA_TEST_SIGNED_INSTALLER` path for local verification of a generated installer.
Read its adjacent `.sig` file and the configured public key, then test successful download and rejection of changed bytes without calling installation.
Record whether the isolated Windows upgrade ran successfully.
If no isolated runner is available during implementation, report that check as unrun instead of claiming the upgrade is verified.

### [x] Task 6: Document setup, recovery, and final verification

Modify `README.md`, `docs/USER_GUIDE.md`, `docs/SETUP.md`, and `docs/RELEASE_CHECKLIST.md`.
Document automatic checks, manual installation, busy work, the first manual update, and recovery through GitHub Releases.
Explain that update signatures protect the update download but do not provide Windows publisher certificate signing.
Document the private key location, required secure backup, GitHub secret names, and local signed-build environment setup.
Keep actual secret contents out of examples, tool output, and Git.
Document unsigned development builds with updater artifacts disabled as a local build override.

Run the final project checks once after all changes:

```powershell
npm.cmd run check
npm.cmd run test
npm.cmd run build
node --test scripts/prepare-update-assets.test.mjs scripts/verify-release-policy.test.mjs
npm.cmd run verify:release-policy
cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
cargo test --manifest-path src-tauri/Cargo.toml --locked
python -m pytest src-tauri/python/tests -v
git diff --check
```

Build one signed Windows installer using the key outside the repository.
Set `AYWA_TEST_SIGNED_INSTALLER` to that installer and run the focused updater download test from Task 2.
Inspect the generated metadata and signature fixture results.
Report the Windows upgrade test separately from unit checks and the local installer build.
Review Git status for accidental key files, generated resources, and unrelated changes.
Mark a task complete only when its change and required checks exist.
Leave commits to the final commit stage.

## Implementation notes and verification

Implemented in the current checkout from commit `648c859d4813094a571a6bbd0d25eae65109bcab`.
The starting changes were the untracked spec and this plan. No other pre-existing changes were present.
No release, tag, remote push, or signing-secret upload was performed. Local commits are handled in the final commit stage.

Routine choices made during implementation:

- The scheduler uses one Rust thread with `tauri::async_runtime::block_on` and a 24-hour sleep. This avoids another timer dependency and follows the app's existing background-thread pattern.
- The plugin's parsed version already exposes `pre.is_empty()`, so the stable-version filter uses that field without adding a version-parser dependency.
- `src-tauri/build.rs` declares the Common Controls v6 manifest dependency. The native mock-app test executable otherwise failed to load with Windows error `0xc0000139` because `TaskDialogIndirect` was unavailable from Common Controls v5. The production installer and all test executables build with this dependency.
- `.gitignore` excludes the generated `release-assets` directory. Local asset verification used a directory under the already ignored `src-tauri/target` because the normal bundle folder contained older installers; the asset script correctly rejected that mixed folder.

Checks completed on Windows on 2026-10-11:

- Locked Rust production check passed. Full Rust suite passed: 107 tests, including eight updater tests.
- Frontend type check passed with no errors or warnings; five test files and 17 tests passed; production frontend build passed.
- Node release-asset and release-policy tests passed: six tests. Release policy passed.
- Python tests passed: seven tests. Rust format check and Git whitespace check passed.
- Workflow YAML and all embedded PowerShell commands parsed successfully. The standalone upgrade script also parsed successfully.
- A signed `Aywa_0.3.0_x64-setup.exe` and its signature were built with the matching private key outside the repository. The real updater download accepted that installer and rejected changed bytes. All four update assets were generated locally, with a matching version, release URL, and signature.
- The footer was inspected in the browser preview on App, Settings, Rules, History, and Debug at 520 by 460 pixels. It fit the viewport and retained keyboard access and the author/source links. The preview has no native Tauri backend, so native update controls and installer handoff were verified through tests rather than an interactive installed app.

Checks still requiring release infrastructure:

- The isolated Windows installation/upgrade/restart/data-preservation smoke test is implemented in the release workflow but has not run locally. It explicitly refuses to run outside an isolated GitHub-hosted Windows runner. Do not treat the signed build or download test as proof that an upgrade ran.
- The interactive footer-to-installer update check requires published newer update assets and has not run. CI checks the real download/signature and NSIS upgrade path separately.
- The maintainer must securely back up the generated key and configure GitHub Actions signing secrets before publishing the first updater-enabled release. Task completion covers code, documentation, and available local checks; it does not imply those external setup steps or publication are complete.

The release build reports an existing `unused_mut` warning in `src-tauri/src/python_runtime.rs:177`; this file was not changed for the updater.

## Final review

An independent reviewer checked the full task change against the spec, existing code and ADR, and simplicity requirements. It found one material issue: CI cleanup waited for the NSIS uninstaller's launcher, which can exit before its temporary child finishes.

Both uninstall commands now pass `/S _?=<absolute installation directory>` with `_?=` last and unquoted. Cleanup waits for the real uninstall before removing residual test files, under exact-path checks. The reviewer confirmed the fix and reported no remaining findings.

YAML and PowerShell syntax checks passed after the fix. A harmless local NSIS fixture under `src-tauri/target` confirmed that the corrected call waits for completion, preserves the real uninstaller's exit code, and supports an installation path with spaces. It waited 1,159 ms for a one-second fixture and observed exit code 7 and the completion marker. This fixture did not install Aywa or touch its data. The actual Aywa upgrade check remains unrun until CI.

No product code, dependencies, or useful tests were removed during review. The implementation keeps the updater in Rust and uses the existing frontend IPC APIs, background-worker pattern, and Node test runner.

[NSIS uninstaller command-line documentation](https://nsis.sourceforge.io/Docs/Chapter3.html#3.2.2) describes the `_?=` wait behavior.

## Sources

[Tauri updater documentation](https://v2.tauri.app/plugin/updater/) defines signing, static feeds, and Windows installation.
[Tauri updater source](https://github.com/tauri-apps/plugins-workspace/blob/v2/plugins/updater/src/updater.rs) defines separate verified download and installation calls.
[NSIS installer source](https://github.com/tauri-apps/tauri/blob/tauri-cli-v2.10.1/crates/tauri-bundler/src/bundle/windows/nsis/installer.nsi) defines update and restart flags.
