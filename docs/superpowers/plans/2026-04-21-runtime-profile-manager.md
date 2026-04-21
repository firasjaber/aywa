# Runtime Profile Manager Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the current hardcoded CUDA-or-CPU worker bootstrap with a manifest-driven runtime profile manager that can auto-resolve, provision, smoke-test, activate, and manually switch Python runtime profiles without blocking the Settings UI.

**Architecture:** Keep runtime-profile logic as its own Rust subsystem, separate from the existing model-manager persistence and model lifecycle code. Reuse the current app-managed embedded Python runtime and worker process entrypoints, but move profile selection, install sequencing, smoke testing, and persisted runtime-profile state behind a dedicated service that `transcribe.rs`, Tauri commands, and `Settings.svelte` can call.

**Tech Stack:** Tauri v2, Rust, SvelteKit, `serde`, `serde_json`, Windows `nvidia-smi` probing, embedded Python, pip-based profile installs, existing worker Python scripts

---

## Scope Check

This spec is still one subsystem. Backend detection, manifest resolution, state persistence, runtime provisioning, worker reporting, and the advanced Settings section all support one runtime-profile manager and should stay in one implementation plan.

## Current Workspace Notes

- `src-tauri/src/transcribe.rs` currently owns runtime bootstrap, hardcodes `cu124` and CPU indices, persists `runtime/worker/bootstrap_state.json`, and probes torch directly.
- `src-tauri/src/model_manager/runtime.rs` already probes NVIDIA presence with `nvidia-smi`; extend or reuse that logic instead of adding a second unrelated GPU detector.
- `src-tauri/src/model_manager/service.rs` already emits the Settings snapshot event and is the narrowest existing UI integration point.
- `src/lib/stores/modelManager.ts` is currently the only Settings data store. Keep using one store for the Settings page snapshot unless that becomes unwieldy.
- `src/lib/components/Settings.svelte` already has runtime status UI and the model action disable rules. Add the runtime-profile controls there in a separate advanced section instead of creating a second settings screen.
- There is no frontend test harness beyond `npm run check`. Keep frontend verification there plus manual Tauri verification.

## Simplicity Decisions Locked In

1. Keep one active installed runtime profile at a time.
2. Keep runtime-profile persistence separate on disk from model registry state.
3. Keep Tauri/UI snapshot delivery integrated with the existing `model-manager-changed` event to minimize frontend churn.
4. Keep worker Python source files bundled as they are; only add the smallest probe or smoke-test helpers needed.
5. Keep pip installation manifest-driven, not solver-driven.
6. Keep CPU as the guaranteed final fallback in `auto` mode.
7. Keep manual profile apply immediate and visible, with no silent fallback to another profile.

## File Map

### Rust backend

- Create: `src-tauri/src/runtime_profile.rs`
  Runtime-profile module root and public exports.
- Create: `src-tauri/src/runtime_profile/types.rs`
  Shared runtime-profile enums, persisted state types, snapshot types, machine facts, smoke-test result, and progress statuses.
- Create: `src-tauri/src/runtime_profile/catalog.rs`
  Bundled manifest loading and typed profile definitions.
- Create: `src-tauri/src/runtime_profile/detect.rs`
  Machine fact detection using `nvidia-smi`, CUDA driver visibility, and compute capability extraction.
- Create: `src-tauri/src/runtime_profile/resolve.rs`
  Auto-mode candidate filtering, ranking, and skip-reason logging helpers.
- Create: `src-tauri/src/runtime_profile/state.rs`
  JSON persistence for `state/runtime_profile_state.json` and manifest/version stamp checks.
- Create: `src-tauri/src/runtime_profile/provision.rs`
  Profile install/reinstall commands, one-operation-at-a-time guard, and worker dependency install helpers.
- Create: `src-tauri/src/runtime_profile/smoke_test.rs`
  Real runtime validation command builder and result parsing.
- Create: `src-tauri/src/runtime_profile/service.rs`
  App-facing orchestration for startup reconcile, apply-now, test-profile, re-detect, reset-to-auto, active-profile reporting, and UI snapshot composition.
- Modify: `src-tauri/src/lib.rs`
  Export the new runtime-profile module and initialize shared state if needed.
- Modify: `src-tauri/src/main.rs`
  Register runtime-profile Tauri commands and startup reconcile wiring.
- Modify: `src-tauri/src/transcribe.rs`
  Remove hardcoded torch index logic, delegate profile preparation/probe work to the runtime-profile service, and report actual execution device/profile metadata.
- Modify: `src-tauri/src/model_manager/runtime.rs`
  Move common NVIDIA probing helpers here or into `runtime_profile/detect.rs` so GPU detection is not duplicated.
- Modify: `src-tauri/src/model_manager/service.rs`
  Include runtime-profile snapshot data in the existing Settings event payload and gate model actions from runtime-profile progress/valid-active-profile state.
- Modify: `src-tauri/src/model_manager/types.rs`
  Add shared snapshot structs only if they truly need to cross the model-manager boundary; otherwise keep runtime-profile types local.

### Python worker runtime

- Modify: `src-tauri/python/worker_main.py`
  Emit execution-device/runtime metadata if the Rust side needs it from worker startup.
- Modify: `src-tauri/python/transcriber.py`
  Keep the worker’s real device selection behavior but surface enough information to distinguish GPU success from CPU fallback for snapshot/logging.
- Create: `src-tauri/python/runtime_smoke_test.py`
  Minimal real smoke-test script that imports torch, captures metadata, optionally runs a CUDA tensor op, and returns structured JSON.

### Frontend

- Modify: `src/lib/stores/modelManager.ts`
  Extend the snapshot types with runtime-profile state, command wrappers, and progress states.
- Modify: `src/lib/components/Settings.svelte`
  Add the advanced runtime-profile section, mode toggle, manual profile picker, progress/result display, and updated model-action disable logic.

### Docs

- Create: `docs/runtime-profile-smoke-checks.md`
  Manual verification commands for auto resolution, manual apply, smoke-test failures, and log inspection.

## Shared Names To Keep Consistent

Use these names consistently across tasks.

```rust
pub const RUNTIME_PROFILE_STATE_FILE: &str = "runtime_profile_state.json";

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeSelectionMode {
    Auto,
    Manual,
}

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeProfileOperationStatus {
    Idle,
    Detecting,
    Resolving,
    Provisioning,
    SmokeTesting,
    Ready,
    Error,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct RuntimeProfileState {
    pub selection_mode: RuntimeSelectionMode,
    pub selected_profile_id: Option<String>,
    pub resolved_auto_profile_id: Option<String>,
    pub active_profile_id: Option<String>,
    pub last_successful_profile_id: Option<String>,
    pub manifest_version: Option<String>,
    pub last_test_result: Option<RuntimeSmokeTestResult>,
    pub last_error: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct RuntimeSmokeTestResult {
    pub profile_id: String,
    pub success: bool,
    pub device: String,
    pub torch_version: Option<String>,
    pub cuda_version: Option<String>,
    pub gpu_name: Option<String>,
    pub tested_at_epoch_ms: u64,
    pub error: Option<String>,
}
```

```ts
export type RuntimeSelectionMode = 'auto' | 'manual';

export type RuntimeProfileOperationStatus =
	| 'idle'
	| 'detecting'
	| 'resolving'
	| 'provisioning'
	| 'smoke_testing'
	| 'ready'
	| 'error';

export type RuntimeProfileSnapshot = {
	selectionMode: RuntimeSelectionMode;
	selectedProfileId: string | null;
	resolvedAutoProfileId: string | null;
	activeProfileId: string | null;
	lastSuccessfulProfileId: string | null;
	manifestVersion: string | null;
	actualExecutionDevice: string | null;
	operationStatus: RuntimeProfileOperationStatus;
	operationMessage: string | null;
	lastError: string | null;
	lastTestResult: RuntimeSmokeTestResult | null;
	profiles: RuntimeProfileCatalogEntry[];
	availableActions: {
		canApplyNow: boolean;
		canTestProfile: boolean;
		canRedetect: boolean;
	};
};
```

### Task 1: Create The Runtime-Profile Module Skeleton And Persisted State

**Files:**
- Create: `src-tauri/src/runtime_profile.rs`
- Create: `src-tauri/src/runtime_profile/types.rs`
- Create: `src-tauri/src/runtime_profile/state.rs`
- Modify: `src-tauri/src/lib.rs`
- Test: `src-tauri/src/runtime_profile/state.rs`

- [ ] Define the shared runtime-profile types and a `RuntimeProfileState::default()` that starts in `auto` mode with no active profile.
- [ ] Write a persistence round-trip unit test in `src-tauri/src/runtime_profile/state.rs` that saves a populated state to a temp file, reloads it, and asserts all required fields survive.
- [ ] Implement `load_state(path: &Path) -> anyhow::Result<RuntimeProfileState>` and `save_state(path: &Path, state: &RuntimeProfileState) -> anyhow::Result<()>` using JSON.
- [ ] Add a helper that resolves the new state file path to `state/runtime_profile_state.json` under app-local storage.
- [ ] Export the new module from `src-tauri/src/lib.rs` and confirm `cargo test runtime_profile::state::tests -- --nocapture` passes.
- [ ] Commit with `git commit -m "feat: add runtime profile state persistence"`.

### Task 2: Add The Bundled Manifest Catalog

**Files:**
- Create: `src-tauri/src/runtime_profile/catalog.rs`
- Create: `src-tauri/src/runtime_profile/manifest.runtime-profiles.json`
- Test: `src-tauri/src/runtime_profile/catalog.rs`

- [ ] Create the bundled manifest file with the initial profile families from the spec: `cpu`, `nvidia-legacy`, `nvidia-current`, and `nvidia-latest`, each with exact package pins, install source, stability, auto eligibility, and compatibility hints.
- [ ] Write a unit test that loads the manifest and asserts those four `profile_id` values exist and that `cpu` is auto-eligible.
- [ ] Implement manifest loading with `include_str!` so the shipped build has one versioned source of truth.
- [ ] Add a `manifest_version()` helper based on manifest content hashing or an explicit version field so startup can detect manifest drift.
- [ ] Add a unit test that rejects malformed compatibility metadata and preserves profile order for deterministic ranking.
- [ ] Run `cargo test runtime_profile::catalog::tests -- --nocapture`.
- [ ] Commit with `git commit -m "feat: add runtime profile manifest catalog"`.

### Task 3: Implement Machine Detection And Auto Candidate Ranking

**Files:**
- Create: `src-tauri/src/runtime_profile/detect.rs`
- Create: `src-tauri/src/runtime_profile/resolve.rs`
- Modify: `src-tauri/src/model_manager/runtime.rs`
- Test: `src-tauri/src/runtime_profile/detect.rs`
- Test: `src-tauri/src/runtime_profile/resolve.rs`

- [ ] Extract or reuse the current `nvidia-smi` parsing so detection returns vendor, GPU name, driver version, and compute capability when available.
- [ ] Add a detector unit test that parses representative NVIDIA outputs and leaves fields empty when `nvidia-smi` is missing.
- [ ] Define the ranking rules in `resolve.rs`: filter to `auto` profiles, skip incompatible capability ranges, prefer the narrowest compatible NVIDIA profile, and always keep `cpu` last.
- [ ] Add unit tests for representative capability cases: older GPU chooses `nvidia-legacy`, mainstream GPU chooses `nvidia-current`, newest GPU chooses `nvidia-latest`, and unknown/unsupported hardware falls back to `cpu`.
- [ ] Add grep-friendly skip and selection reason strings so later orchestration can log `reason=...` without rebuilding explanations.
- [ ] Run `cargo test runtime_profile::detect::tests runtime_profile::resolve::tests -- --nocapture`.
- [ ] Commit with `git commit -m "feat: add runtime profile detection and resolver"`.

### Task 4: Move Provisioning Out Of `transcribe.rs` And Make It Profile-Driven

**Files:**
- Create: `src-tauri/src/runtime_profile/provision.rs`
- Modify: `src-tauri/src/transcribe.rs`
- Test: `src-tauri/src/runtime_profile/provision.rs`

- [ ] Copy the current pip/bootstrap helpers out of `transcribe.rs` into `runtime_profile/provision.rs` without changing behavior yet.
- [ ] Replace the hardcoded `TORCH_CUDA_INDEX_URL` and `TORCH_CPU_INDEX_URL` install path with manifest-driven profile install steps.
- [ ] Add a provisioning test that verifies the generated pip command for a profile uses that profile’s `install_source` and exact package pins.
- [ ] Add a single-operation guard so only one runtime-profile background job can run at a time; return a visible error if the user tries to start another operation mid-flight.
- [ ] Update `transcribe.rs` to call `runtime_profile::service::ensure_active_profile(...)` instead of its own `ensure_worker_dependencies(...)` path.
- [ ] Remove or deprecate `runtime/worker/bootstrap_state.json` in favor of runtime-profile state plus manifest stamp.
- [ ] Run `cargo test runtime_profile::provision::tests -- --nocapture`.
- [ ] Commit with `git commit -m "refactor: make worker runtime provisioning profile-driven"`.

### Task 5: Add The Real Smoke Test And Auto Fallback Sequencing

**Files:**
- Create: `src-tauri/src/runtime_profile/smoke_test.rs`
- Create: `src-tauri/python/runtime_smoke_test.py`
- Create: `src-tauri/src/runtime_profile/service.rs`
- Modify: `src-tauri/src/transcribe.rs`
- Test: `src-tauri/src/runtime_profile/service.rs`

- [ ] Create `runtime_smoke_test.py` so it outputs one JSON payload with torch version, CUDA version, GPU name, success/failure, and an actual CUDA tensor-op result when the profile targets GPU.
- [ ] Add a smoke-test parser unit test that accepts success output and preserves exact error text from failure output.
- [ ] Implement service orchestration for startup reconcile: detect machine facts, load manifest, resolve candidates, provision the best profile, run smoke test, activate the first passing profile, and fall back through candidates until `cpu` if needed.
- [ ] Add a unit test for auto mode where the first NVIDIA candidate fails smoke test, the second passes, and the final state records both the active profile and last successful profile.
- [ ] Add a unit test for manual mode where the selected profile fails smoke test and the service leaves the app in visible failure state without switching to another profile.
- [ ] Update `transcribe.rs` worker startup to record the actual execution device from the smoke-test result or worker startup metadata rather than from `torch.cuda.is_available()` alone.
- [ ] Run `cargo test runtime_profile::service::tests -- --nocapture`.
- [ ] Commit with `git commit -m "feat: add runtime smoke tests and fallback sequencing"`.

### Task 6: Wire Runtime-Profile Snapshot And Commands Through Tauri

**Files:**
- Modify: `src-tauri/src/main.rs`
- Modify: `src-tauri/src/lib.rs`
- Modify: `src-tauri/src/model_manager/service.rs`
- Modify: `src-tauri/src/model_manager/types.rs`
- Modify: `src/lib/stores/modelManager.ts`

- [ ] Add Tauri commands for `set_runtime_selection_mode`, `select_runtime_profile`, `apply_runtime_profile_now`, `test_runtime_profile`, `redetect_runtime_profile`, and `reset_runtime_profile_to_auto`.
- [ ] Add the runtime-profile snapshot into the existing `ModelManagerSnapshot` payload so the frontend can keep using one event subscription.
- [ ] Derive `readyForTranscription` from both model readiness and runtime-profile readiness: model actions should be disabled only while reprovisioning is active or when no valid active profile exists.
- [ ] Extend `src/lib/stores/modelManager.ts` with the new snapshot types and invoke wrappers for each runtime-profile command.
- [ ] Run `cargo test -- --nocapture` for backend compile coverage and `npm run check` for frontend typing coverage.
- [ ] Commit with `git commit -m "feat: expose runtime profile state to tauri and frontend"`.

### Task 7: Add The Advanced Runtime Section In `Settings.svelte`

**Files:**
- Modify: `src/lib/components/Settings.svelte`
- Test: `npm run check`

- [ ] Add a visually separate advanced runtime-profile section below the current runtime summary and above or beside the model rows.
- [ ] Render selection mode, resolved auto profile, active installed profile, actual execution device, last smoke-test result, last test timestamp, and current operation progress.
- [ ] Add the mode toggle, manual profile picker, `Apply now`, `Test profile`, `Re-detect`, and `Reset to auto` controls, all wired to the new store actions.
- [ ] Keep the page responsive by leaving controls outside the active operation enabled when safe; only disable controls that would conflict with the current background job.
- [ ] Update the model row action disable logic to reflect runtime-profile operation state and missing active profile state.
- [ ] Run `npm run check` and manually verify the page still works on narrow mobile width.
- [ ] Commit with `git commit -m "feat: add settings runtime profile controls"`.

### Task 8: Add Structured Logging And Manual Verification Docs

**Files:**
- Modify: `src-tauri/src/runtime_profile/detect.rs`
- Modify: `src-tauri/src/runtime_profile/resolve.rs`
- Modify: `src-tauri/src/runtime_profile/provision.rs`
- Modify: `src-tauri/src/runtime_profile/service.rs`
- Modify: `src-tauri/src/transcribe.rs`
- Create: `docs/runtime-profile-smoke-checks.md`

- [ ] Add structured `eprintln!` lines for detect, resolve, skip, install start/fail, smoke-test start/fail, activate success, manual apply, and CPU fallback outcomes using the `[runtime-profile] ...` prefix from the spec.
- [ ] Make sure every logged branch includes `profile_id=...` or an explicit `reason=...` so failures stay grep-friendly.
- [ ] Document manual verification flows in `docs/runtime-profile-smoke-checks.md` for: auto on a compatible GPU, auto fallback to CPU, manual incompatible profile failure, `Reset to auto`, and log inspection.
- [ ] Run `cargo test -- --nocapture`, `npm run check`, and one manual Tauri session to confirm logs and Settings state match the spec.
- [ ] Commit with `git commit -m "docs: add runtime profile verification guide"`.

## Final Verification Checklist

- `cargo test -- --nocapture`
- `npm run check`
- Manual Tauri verification on one machine with no NVIDIA GPU or forced CPU profile
- Manual Tauri verification on one NVIDIA machine where at least one GPU profile is eligible
- Log grep checks for `"[runtime-profile] detect"`, `"[runtime-profile] resolve"`, `"[runtime-profile] smoke-test"`, and `"[runtime-profile] activate success"`

## Spec Coverage Review

- Manifest-driven catalog: Tasks 2, 3, 4.
- Separate persisted runtime-profile state: Task 1.
- Auto detection and ranking by GPU facts and compute capability: Task 3.
- Provisioning, smoke test, activation, and fallback sequencing: Tasks 4 and 5.
- Manual switching and visible failure behavior: Tasks 5, 6, and 7.
- Settings responsiveness and advanced runtime section: Tasks 6 and 7.
- Logging and debugging support: Task 8.
- Actual execution-device reporting: Tasks 5, 6, and 7.
- Storage layout additions under `state/` and `runtime/`: Tasks 1 and 4.
- Backend/frontend/manual testing strategy: Tasks 3, 5, 6, 7, 8, and final checklist.

Plan complete and saved to `docs/superpowers/plans/2026-04-21-runtime-profile-manager.md`. Two execution options:

1. Subagent-Driven (recommended) - I dispatch a fresh subagent per task, review between tasks, fast iteration
2. Inline Execution - Execute tasks in this session using executing-plans, batch execution with checkpoints

Which approach?
