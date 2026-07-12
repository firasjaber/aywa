# Unify CUDA Status Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make the App and Debug tabs report the same live CUDA status from the Python worker that actually loads the model.

**Architecture:** Keep the embedded Python runtime probe as the authority for CUDA, PyTorch, and GPU details. Record the `device` reported by the running worker when it successfully loads a model, and derive the ready banner from that live state rather than the persisted result of a previous smoke test. Retain `nvidia-smi`/`nvcuda.dll` detection only as optional system metadata; its failure must not contradict the worker's confirmed CUDA state.

**Tech Stack:** Rust 2021, Tauri 2, embedded Python/PyTorch worker, Svelte 5, TypeScript.

---

## File structure

- `src-tauri/src/model_manager/types.rs` — add a serializable live execution-device field to the model-manager snapshot/state.
- `src-tauri/src/model_manager/service.rs` — preserve the worker's reported device and compute readiness from it and the live worker CUDA probe.
- `src-tauri/src/transcribe.rs` — return the device from worker startup to the model manager, and add protocol/status tests.
- `src/lib/stores/modelManager.ts` — model the new snapshot field and its safe initial value.
- `src/lib/components/settingsView.ts` — render the ready banner from the live device confirmation instead of persisted runtime-profile history.
- `src/lib/components/Settings.svelte` — label system metadata as supplemental and present the live worker status without implying a failed `nvidia-smi` check means CUDA is unusable.

### Task 1: Carry the loaded worker device through the backend

**Files:**
- Modify: `src-tauri/src/transcribe.rs`
- Modify: `src-tauri/src/model_manager/types.rs`
- Test: `src-tauri/src/transcribe.rs`

- [ ] **Step 1: Write the failing worker protocol test**

Add a unit test which parses a worker-ready response and asserts the device is retained:

```rust
#[test]
fn ready_response_keeps_reported_device() {
    let response = decode_worker_response_line(
        r#"{\"type\":\"ready\",\"model_id\":\"parakeet-v0.2-en\",\"model_path\":\"C:/models/model.nemo\",\"device\":\"cuda\"}"#,
    )
    .unwrap();

    assert!(matches!(
        response,
        WorkerResponse::Ready { device: Some(device), .. } if device == "cuda"
    ));
}
```

- [ ] **Step 2: Run the failing test**

Run: `cargo test ready_response_keeps_reported_device --manifest-path src-tauri/Cargo.toml`

Expected: PASS after the existing protocol is used as the implementation reference; if it fails, correct only the JSON fixture or response deserialization.

- [ ] **Step 3: Add live execution state to the model-manager types**

Add an optional `live_execution_device: Option<String>` field to `ModelManagerState` and `ModelManagerSnapshot`. Initialize it to `None`; it means no currently loaded worker has confirmed a device. Keep the field camel-cased when serialized by the existing snapshot serde configuration.

```rust
pub live_execution_device: Mutex<Option<String>>,

pub live_execution_device: Option<String>,
```

- [ ] **Step 4: Return the device from successful worker startup**

Change the successful `ensure_loaded` path so it returns the `device` supplied by `WorkerResponse::Ready`, normalized with the existing `normalize_worker_device` behavior (`cuda` → `gpu`, `cpu` → `cpu_fallback`). If the worker omits `device`, return `None`; do not infer GPU from the historical smoke-test state.

```rust
let device = match wait_for_worker_ready(&mut worker)? {
    WorkerResponse::Ready { device, .. } => device.as_deref().map(normalize_worker_device),
    _ => None,
};
```

- [ ] **Step 5: Verify backend compilation and focused tests**

Run: `cargo test --manifest-path src-tauri/Cargo.toml transcribe::tests`

Expected: all `transcribe::tests` pass.

- [ ] **Step 6: Commit the backend protocol/state change**

```bash
git add src-tauri/src/transcribe.rs src-tauri/src/model_manager/types.rs
git commit -m "track live worker execution device"
```

### Task 2: Make snapshots and readiness use current worker evidence

**Files:**
- Modify: `src-tauri/src/model_manager/service.rs`
- Test: `src-tauri/src/model_manager/service.rs`

- [ ] **Step 1: Write snapshot behavior tests**

Add focused tests around `snapshot_from_registry` (extract a small pure helper if needed) proving:

```rust
#[test]
fn live_gpu_worker_overrides_historical_cpu_profile_for_display() {
    assert_eq!(effective_execution_device(Some("gpu"), Some("cpu_fallback")), Some("gpu"));
}

#[test]
fn no_loaded_worker_has_no_live_execution_device() {
    assert_eq!(effective_execution_device(None, Some("gpu")), None);
}
```

- [ ] **Step 2: Run the new tests to verify they fail**

Run: `cargo test effective_execution_device --manifest-path src-tauri/Cargo.toml`

Expected: FAIL because the pure helper does not exist yet.

- [ ] **Step 3: Store the ready worker's device and refresh its CUDA probe**

After `transcribe::ensure_loaded` succeeds, set `live_execution_device` from its returned device. Then run `transcribe::probe_worker_cuda(app)` and replace `worker_cuda_status` when the probe succeeds, so the Debug tab reflects the same Python runtime used by the worker. If the refresh itself fails, retain the prior runtime status and log the refresh error; loading an already-ready worker must not be made to fail by diagnostics refresh.

Clear `live_execution_device` before emitting snapshots when the worker unloads, model cache clears, or model startup fails.

- [ ] **Step 4: Put live evidence in the snapshot**

Clone `live_execution_device` in `ModelManagerState::snapshot`, add it to `snapshot_from_registry`, and serialize it as `liveExecutionDevice`. Compute `ready_for_transcription` only when a model is loaded, the bootstrap is ready, and `live_execution_device` is present.

- [ ] **Step 5: Verify snapshot behavior and the Rust suite**

Run: `cargo test --manifest-path src-tauri/Cargo.toml`

Expected: all Rust unit tests pass.

- [ ] **Step 6: Commit the backend status unification**

```bash
git add src-tauri/src/model_manager/service.rs
git commit -m "use live worker status for CUDA diagnostics"
```

### Task 3: Render a single truthful status in Settings

**Files:**
- Modify: `src/lib/stores/modelManager.ts`
- Modify: `src/lib/components/settingsView.ts`
- Modify: `src/lib/components/Settings.svelte`

- [ ] **Step 1: Extend the frontend snapshot type and initial state**

Add `liveExecutionDevice: string | null` to `ModelManagerSnapshot` and set `liveExecutionDevice: null` in `initialState`.

- [ ] **Step 2: Use the live device for the ready card**

Replace the `runtimeProfile.actualExecutionDevice` checks in `settingsView.ts` with `snapshot.liveExecutionDevice`. Preserve the existing GPU/CPU card copy, but when the value is `null`, do not claim a device is active.

```ts
if (snapshot.liveExecutionDevice === 'gpu') {
  return { tone: 'positive', title: 'Ready to dictate', detail: 'NVIDIA GPU acceleration is active.', badge: 'GPU', showHotkey: true };
}
```

- [ ] **Step 3: Clarify the Debug panel's source of truth**

Change the `System` label/value to explain it is an optional Windows driver-tool check. Keep the worker-probe `Acceleration` row as the CUDA authority, and append the live worker device when present. Do not show a driver-tool detection failure as proof that CUDA is unavailable.

- [ ] **Step 4: Run frontend type checking**

Run: `npm.cmd run check`

Expected: `svelte-check found 0 errors and 0 warnings`.

- [ ] **Step 5: Build the production frontend**

Run: `npm.cmd run build`

Expected: Vite completes successfully.

- [ ] **Step 6: Commit the settings display change**

```bash
git add src/lib/stores/modelManager.ts src/lib/components/settingsView.ts src/lib/components/Settings.svelte
git commit -m "show CUDA status from live worker"
```

### Task 4: Verify the end-to-end contradiction is eliminated

**Files:**
- Modify: none

- [ ] **Step 1: Start the app with its embedded runtime**

Run: `npm.cmd run tauri:dev:managed-python`

Expected: the runtime reaches `Ready` and Settings can be opened.

- [ ] **Step 2: Load the Parakeet model and inspect both tabs**

Expected: when the worker reports `cuda`, App displays the GPU badge and Debug displays `CUDA available` with a live GPU execution device. When it reports `cpu`, App displays the CPU badge and Debug does not claim GPU execution.

- [ ] **Step 3: Confirm driver-tool failure is not treated as CUDA failure**

Expected: if `nvidia-smi` or direct `nvcuda.dll` loading fails, the System row labels this as supplemental detection while the Acceleration row retains the live worker result.

- [ ] **Step 4: Confirm unload resets live device state**

Expected: after unloading the model, `liveExecutionDevice` becomes null and the App tab stops presenting GPU or CPU execution as active.
