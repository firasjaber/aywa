# Runtime Profile Manager Design

## Summary

The app should manage Python runtime compatibility through a small catalog of app-defined runtime profiles instead of hardcoding a single PyTorch CUDA wheel. The default behavior stays `auto`, where the app detects the user's GPU, selects the best profile from a bundled manifest, provisions it into the app-managed Python runtime, runs a real smoke test, and activates it only if the profile actually works.

The system must also support immediate manual switching for testing. In `manual` mode, the user can pick a profile from Settings, apply it immediately, and inspect the result without blocking the UI. Logging is a primary debugging surface, so every detection, resolution, provisioning, smoke-test, activation, and fallback decision must emit clear, grep-friendly logs.

## Goals

- Replace the current single hardcoded CUDA wheel strategy with a manifest-driven runtime profile system.
- Keep CPU as the guaranteed baseline fallback.
- In `auto`, choose the best profile for the detected machine based on GPU facts, especially compute capability.
- Support immediate manual switching so runtime profiles can be tested on the fly.
- Keep Settings responsive during provision/reprovision/test work.
- Make logs detailed enough to diagnose profile selection and activation failures quickly.

## Non-Goals

- No generalized dependency solver.
- No remote compatibility database.
- No parallel installation of multiple profiles.
- No large UI surface beyond one advanced runtime section.
- No silent manual-mode profile hopping; manual mode should fail visibly if the chosen profile fails.

## Current Problem

The app currently treats Python runtime provisioning as a single path that prefers one CUDA wheel and then falls back to CPU. That works for some machines, but it fails for newer or older GPUs whose supported PyTorch/CUDA combinations differ. Machine-level CUDA detection is not enough because a GPU can be visible to PyTorch while still being incompatible with the installed wheel or workload.

The immediate example is a modern NVIDIA GPU with compute capability `sm_120` being detected correctly, but the installed `torch 2.6.0+cu124` wheel only supporting kernels through `sm_90`. In that state:

- `torch.cuda.is_available()` returns `True`
- worker startup on GPU fails
- the correct fix is a different runtime profile, not a generic reinstall of CUDA

## Proposed Approach

Use a curated runtime profile catalog with two user-facing selection modes:

- `auto`: default mode; the app resolves the best candidate profile from the manifest based on detected machine facts and verifies it with a smoke test
- `manual`: advanced mode; the user selects a specific profile and the app applies it immediately for testing

Each runtime profile defines a tested package set and installation source. The app installs exactly one active profile into the app-managed Python runtime at a time.

## Architecture

Add a runtime-profile subsystem alongside the model manager.

Core components:

1. Runtime profile catalog
   - Bundled, versioned manifest shipped with the app.
   - Defines available profiles and compatibility metadata.

2. Runtime profile state
   - Persists user selection mode, selected profile, active profile, last successful profile, and last test result.

3. Runtime profile resolver
   - In `auto`, ranks eligible manifest entries against detected GPU and runtime facts.

4. Runtime profile provisioner
   - Installs or reinstalls the selected profile into the app-managed Python runtime.

5. Runtime smoke tester
   - Performs a real validation pass after provisioning.

6. Worker runtime launcher
   - Starts the transcription worker using the active installed profile and reports actual execution behavior.

Boundary rule:

- Model manager owns speech-model lifecycle.
- Runtime profile manager owns Python runtime selection, provisioning, and validation.

## Runtime Profile Manifest

The manifest is bundled with the app and versioned with the build.

Each profile entry should contain:

- `profile_id`
- `label`
- `stability` such as `stable`, `preview`, or `legacy`
- `install_source` or wheel index URL
- exact package pins
- compatibility hints
- supported compute capability range or allowlist when known
- optional minimum driver/runtime requirements
- whether the profile is eligible for `auto`

Example profile families:

- `cpu`
- `nvidia-legacy`
- `nvidia-current`
- `nvidia-latest`
- optional preview/nightly profile for the newest GPUs

The manifest is the source of truth. The app should not hardcode one CUDA wheel path in provisioning logic once this system lands.

## Machine Detection And Auto Resolution

In `auto`, the app should select the correct manifest entry based on detected machine facts.

Detected facts should include, when available:

- GPU vendor
- GPU name
- compute capability
- NVIDIA driver visibility/version
- current Python runtime facts relevant to validation

Auto resolution flow:

1. Detect machine facts.
2. Load the bundled runtime profile manifest.
3. Filter to profiles eligible for `auto`.
4. Rank candidates against detected GPU facts, especially compute capability.
5. Try the best candidate first.
6. Run a smoke test.
7. If it fails, try the next compatible manifest candidate.
8. Activate the first profile that passes.
9. If all GPU candidates fail, use CPU fallback.

Important rule:

- `auto` should prefer a profile chosen from the manifest for the detected GPU.
- It must not rely only on `torch.cuda.is_available()`.

## Persistent State

Persist runtime-profile state separately from speech-model state.

Required fields:

- `selection_mode`: `auto` or `manual`
- `selected_profile_id`: user-selected profile in manual mode
- `resolved_auto_profile_id`: current manifest-selected profile for this machine
- `active_profile_id`: currently installed usable profile
- `last_successful_profile_id`
- `last_test_result`
- `last_error`
- manifest/version stamp used for the active profile

Behavior rules:

- In `auto`, do not stick forever to an outdated profile if the manifest changes.
- Also do not reprovision unnecessarily on every startup.
- Reprovision in `auto` when:
  - no active profile exists
  - the active profile no longer matches the current manifest/version
  - the user explicitly requests retest/reprovision
  - the previous smoke test failed

## Provisioning And Activation Workflow

Only one runtime-profile background operation should run at a time.

Supported operations:

- initial provision
- reprovision after profile switch
- smoke test
- re-detect and re-resolve in `auto`

Provisioning flow:

1. Resolve target profile.
2. Unload any active worker if the runtime is being replaced.
3. Install the target profile into the app-managed Python runtime.
4. Run smoke test.
5. If success, mark profile active.
6. If auto mode and the profile fails, try the next candidate.
7. If manual mode and the profile fails, stop and report the failure.

The app-managed embedded Python runtime should remain the only runtime used by the worker. The app should not mutate the user's global Python as part of normal shipped behavior.

## Manual Switching And Testing

Manual switching is intentionally immediate.

When the user changes profile in manual mode:

- the app applies it immediately
- unloads the current worker first if needed
- reprovisions in the background
- runs the smoke test
- updates the UI with success or failure

Manual mode rules:

- never silently switch to another profile if the chosen one fails
- preserve logs and test results for debugging
- allow the user to return to `auto` at any time

## Settings UI

Keep runtime-profile controls in a small advanced section of `Settings.svelte`, separate from the speech model rows.

Display:

- selection mode: `Auto` or `Manual`
- resolved auto profile
- active installed profile
- actual execution device: `GPU`, `CPU fallback`, or `Not tested`
- last smoke-test result and timestamp
- current provisioning/testing progress

Controls:

- mode toggle: `Auto` / `Manual`
- profile picker in manual mode
- `Apply now`
- `Test profile`
- `Re-detect`
- `Reset to auto`

UX rules:

- Keep the UI responsive during provisioning and testing.
- Keep the runtime profile section visually separate from the speech model section.
- Disable model load actions only when runtime reprovisioning is actively in progress or when no valid active profile exists.

## Logging

Logging is a first-class debugging mechanism.

Every runtime-profile action should emit structured, grep-friendly logs.

Examples:

- `[runtime-profile] detect gpu_name=... capability=... driver=...`
- `[runtime-profile] resolve mode=auto selected_profile=... reason=...`
- `[runtime-profile] skip profile_id=... reason=...`
- `[runtime-profile] install start profile_id=...`
- `[runtime-profile] install failed profile_id=... error=...`
- `[runtime-profile] smoke-test start profile_id=...`
- `[runtime-profile] smoke-test failed profile_id=... error=...`
- `[runtime-profile] activate success profile_id=... device=cpu_fallback`
- `[runtime-profile] manual apply profile_id=...`

Logs must make it easy to answer:

- what machine was detected
- what profile was chosen
- why a profile was chosen or skipped
- whether install succeeded
- whether smoke test succeeded
- what profile is actually active now
- whether the worker ended up on GPU or CPU fallback

## Smoke Test

The smoke test must stay small but real.

Minimum checks:

1. import `torch`
2. capture installed wheel/runtime metadata
3. if the profile targets GPU, run a small CUDA tensor operation
4. run a lightweight worker/runtime validation when practical
5. store the exact success or failure result

The smoke test should validate executable behavior, not only importability.

## Failure Policy

Auto mode:

- try the next manifest-compatible profile on failure
- preserve detailed logs for each failed attempt
- keep CPU as the guaranteed final fallback when available

Manual mode:

- do not silently move to another profile
- fail visibly in Settings
- keep the error text and logs available for debugging

Worker startup:

- even when a profile passes runtime selection, worker startup may still fall back to CPU for model-specific reasons
- the app should continue to surface the actual execution device used by the worker

## Storage Layout

Add runtime-profile state under app-local storage, alongside the existing model/runtime folders.

Expected additions:

```text
state/runtime_profile_state.json
runtime/python/
runtime/worker/
runtime/profiles/<profile-id>/...
```

The exact on-disk structure can stay minimal as long as the active profile, manifest/version stamp, and last results are persisted clearly.

## Testing Strategy

Backend tests:

- profile manifest parsing
- auto-profile ranking for representative GPU capabilities
- persistence round-trip for runtime-profile state
- manual-mode failure behavior
- auto-mode multi-profile fallback behavior
- smoke-test result handling

Frontend tests or checks:

- advanced runtime section renders selection mode, active profile, and progress state correctly
- model actions react correctly to active reprovisioning state

Manual verification:

- newer GPU selects `nvidia-latest` or equivalent compatible profile
- older GPU selects `nvidia-legacy` or equivalent compatible profile
- incompatible profile can be selected manually and fails visibly
- `Reset to auto` returns control to manifest-driven selection
- logs are sufficient to explain each outcome

## Incremental Delivery

Keep implementation staged and minimal:

1. Introduce manifest and persisted runtime-profile state.
2. Extract current provisioning logic into a runtime-profile manager with one active profile at a time.
3. Add auto resolution using detected GPU facts.
4. Add smoke test and fallback sequencing.
5. Add advanced Settings controls for manual switching and testing.
6. Add actual active-profile and execution-device reporting.

## Open Decisions Resolved In This Spec

- Use a profile catalog, not one hardcoded CUDA install path.
- Keep `auto` as the default mode.
- In `auto`, select the best manifest candidate based on the detected GPU.
- Support immediate manual apply in an advanced settings section.
- Keep the first version minimal and logging-heavy.
- Use the app-managed embedded Python runtime, not the user's global Python, for shipped behavior.
