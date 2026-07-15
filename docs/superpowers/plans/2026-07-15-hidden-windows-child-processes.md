# Hidden Windows Child Processes Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Prevent every internal subprocess in the packaged Windows app from creating a visible console window while preserving background runtime setup and model activation.

**Architecture:** Add one backend helper that configures a `std::process::Command` for internal Windows work with `CREATE_NO_WINDOW`. Replace each internal `Command::new` call with the helper, leaving the intentionally user-facing Explorer launch unchanged. Existing bootstrap and model-loader threads remain unchanged, so blocking preparation stays outside the UI path.

**Tech Stack:** Rust 2021, Tauri 2, Windows `CommandExt`.

---

## File structure

- Create: `src-tauri/src/child_process.rs` — owns the no-console configuration and platform-gated tests.
- Modify: `src-tauri/src/lib.rs` — exposes the helper module.
- Modify: `src-tauri/src/diagnostics.rs`, `python_runtime.rs`, and `transcribe.rs` — use the helper for Python and pip commands.
- Modify: `src-tauri/src/runtime_profile/detect.rs`, `runtime_profile/smoke_test.rs`, and `model_manager/runtime.rs` — use the helper for GPU detection and smoke tests.

### Task 1: Add the shared Windows child-process helper

**Files:**
- Create: `src-tauri/src/child_process.rs`
- Modify: `src-tauri/src/lib.rs:1-10`
- Test: `src-tauri/src/child_process.rs`

- [ ] **Step 1: Write the failing platform-flag test**

```rust
#[cfg(target_os = "windows")]
#[test]
fn internal_commands_use_the_no_console_creation_flag() {
    assert_eq!(INTERNAL_COMMAND_CREATION_FLAGS, 0x0800_0000);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --manifest-path src-tauri/Cargo.toml child_process::tests::internal_commands_use_the_no_console_creation_flag`

Expected: FAIL because `child_process` and `INTERNAL_COMMAND_CREATION_FLAGS` do not exist.

- [ ] **Step 3: Implement the helper and expose it from the backend crate**

```rust
use std::{ffi::OsStr, process::Command};

#[cfg(target_os = "windows")]
pub const INTERNAL_COMMAND_CREATION_FLAGS: u32 = 0x0800_0000;

pub fn internal_command(program: impl AsRef<OsStr>) -> Command {
    let mut command = Command::new(program);

    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(INTERNAL_COMMAND_CREATION_FLAGS);
    }

    command
}
```

Add `pub mod child_process;` beside the existing module declarations in `src-tauri/src/lib.rs` and add the platform-gated test from Step 1.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --manifest-path src-tauri/Cargo.toml child_process::tests::internal_commands_use_the_no_console_creation_flag`

Expected: PASS on Windows.

- [ ] **Step 5: Commit**

Run: `git add src-tauri/src/child_process.rs src-tauri/src/lib.rs; git commit -m "fix: hide internal Windows child processes"`

### Task 2: Route every internal subprocess through the helper

**Files:**
- Modify: `src-tauri/src/diagnostics.rs:182-188`
- Modify: `src-tauri/src/python_runtime.rs:311-339`
- Modify: `src-tauri/src/transcribe.rs:316-324,391-396,430-439,783-795`
- Modify: `src-tauri/src/runtime_profile/detect.rs:38-45`
- Modify: `src-tauri/src/runtime_profile/smoke_test.rs:21-34`
- Modify: `src-tauri/src/model_manager/runtime.rs:15-19`

- [ ] **Step 1: Establish failing coverage**

Run: `rg -n 'Command::new' src-tauri/src -g '*.rs'`

Expected: output includes Python, `nvidia-smi`, worker, and logged-command call sites in addition to `explorer.exe`.

- [ ] **Step 2: Replace internal command construction with the helper**

At each listed internal call site, replace:

```rust
Command::new(program)
```

with:

```rust
crate::child_process::internal_command(program)
```

For `model_manager/runtime.rs`, import the helper with `use crate::{child_process::internal_command, model_manager::types::RuntimePreflight};`. Remove unused `Command` imports. Do not alter `diagnostics::open_log_in_file_manager`; `explorer.exe` is intentionally user facing.

- [ ] **Step 3: Verify complete coverage**

Run: `rg -n 'Command::new' src-tauri/src -g '*.rs'`

Expected: only `src-tauri/src/child_process.rs` and the intentional `explorer.exe` line in `src-tauri/src/diagnostics.rs` remain.

- [ ] **Step 4: Run tests**

Run: `cargo test --manifest-path src-tauri/Cargo.toml`

Expected: PASS.

- [ ] **Step 5: Commit**

Run: `git add src-tauri/src/diagnostics.rs src-tauri/src/python_runtime.rs src-tauri/src/transcribe.rs src-tauri/src/runtime_profile/detect.rs src-tauri/src/runtime_profile/smoke_test.rs src-tauri/src/model_manager/runtime.rs; git commit -m "fix: suppress console windows during runtime setup"`

### Task 3: Verify the packaged application

**Files:**
- Verify only: `src-tauri/src/main.rs:1`

- [ ] **Step 1: Confirm the release GUI subsystem remains configured**

Run: `rg -n 'windows_subsystem = "windows"' src-tauri/src/main.rs`

Expected: the release-only GUI subsystem attribute is present.

- [ ] **Step 2: Run frontend checks**

Run: `npm run check`

Expected: exits successfully with no Svelte or TypeScript errors.

- [ ] **Step 3: Build the package**

Run: `npx.cmd --no-install tauri build --bundles nsis`

Expected: the NSIS release bundle is produced successfully.

- [ ] **Step 4: Perform a manual packaged-build smoke test**

1. Launch the installed release app; no terminal appears while runtime preparation reports progress.

2. Load a downloaded model; no terminal appears or steals focus while the worker starts.

3. Confirm Settings progress and model status update normally, and use the tray/settings UI while preparation runs.
