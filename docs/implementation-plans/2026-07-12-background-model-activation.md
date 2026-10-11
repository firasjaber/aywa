# Background Model Activation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Activate a transcription model in the background while exposing an authoritative loading state to the UI.

**Architecture:** The model-manager command records `Loading` and returns a snapshot immediately. A spawned Rust thread runs worker startup and emits the final `Loaded` or `Error` snapshot. Svelte derives activation feedback and button disabling from the snapshot.

**Tech Stack:** Rust, Tauri 2 events, Svelte 5, TypeScript, Cargo tests, svelte-check.

---

### Task 1: Make backend activation asynchronous

**Files:**
- Modify: `src-tauri/src/model_manager/service.rs`
- Test: `src-tauri/src/model_manager/service.rs`

- [ ] Add unit coverage proving a transient `Loading` status produces a loading row and blocks a second activation.
- [ ] Run `cargo test model_manager::service --lib` and confirm the new test initially fails.
- [ ] In `load_model`, reject an existing loading status, validate the requested installed model, add `ModelStatus::Loading`, emit a snapshot, and return immediately after spawning a thread.
- [ ] Move the existing worker start, ready wait, CUDA probe, registry persistence, error persistence, loading-status cleanup, and final snapshot emission into a background helper.
- [ ] Run `cargo test model_manager::service --lib` and confirm it passes.

### Task 2: Derive Settings feedback from the backend state

**Files:**
- Modify: `src/lib/components/Settings.svelte`
- Modify: `src/lib/components/settingsView.ts`

- [ ] Replace component-local activation state with the row whose status is `Loading`.
- [ ] Have `selectAppStatus` return progress copy naming that loading model.
- [ ] Show `Preparing...` for the loading row and disable every Activate button while any row is loading.
- [ ] Run `npm.cmd run check` and confirm no Svelte or TypeScript diagnostics.

### Task 3: Verify and commit

**Files:**
- Modify: `src-tauri/src/model_manager/service.rs`
- Modify: `src/lib/components/Settings.svelte`
- Modify: `src/lib/components/settingsView.ts`
- Create: `docs/superpowers/plans/2026-07-12-background-model-activation.md`

- [ ] Run `cargo fmt --check`; if formatting is required, run `cargo fmt` and repeat the check.
- [ ] Run `cargo test --manifest-path src-tauri/Cargo.toml --lib` and `npm.cmd run check`; both must pass.
- [ ] Inspect the focused diff, stage only the activation files and this plan, then commit with message `activate models in the background`.
