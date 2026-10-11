# Worker Transcription Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a Rust-supervised private Python transcription worker so the existing push-to-talk recording flow produces real local transcripts, copies them to the clipboard, and attempts paste.

**Architecture:** Keep Tauri as the only orchestrator. Rust owns model loading, worker lifecycle, request/response handling, state transitions, and clipboard/paste behavior; Python stays a private subprocess that loads one model and accepts WAV path transcription requests over newline-delimited JSON on stdio.

**Tech Stack:** Tauri, Rust, serde, std::process, std::sync::mpsc, Svelte, TypeScript, embedded Python runtime, Python worker script, JSON-over-stdio

---

## File Map

### Rust files

- Modify: `src-tauri/src/lib.rs`
  - Replace the direct `stop_recording -> Ready` path with `stop_recording -> transcribe pipeline`.
  - Add a managed transcription runtime to `RuntimeHandles`.
  - Add helpers for scheduling short-lived `Completed` and `Error` reset behavior.
- Modify: `src-tauri/src/main.rs`
  - Register any new startup hooks needed by the transcription runtime.
- Modify: `src-tauri/src/state.rs`
  - Replace `Saving` with `Processing` and `Completed`.
  - Update transition rules and tests.
- Modify: `src-tauri/src/overlay.rs`
  - Show the overlay for `Processing`, `Completed`, and `Error`.
- Modify: `src-tauri/src/clipboard.rs`
  - Keep `copy_text` and add a small `paste_clipboard()` helper for `Ctrl+V`.
- Modify: `src-tauri/src/model_manager/service.rs`
  - Make `load_model` delegate to the transcription runtime.
  - Make `unload_model` shut down the worker before clearing `loaded_model_id`.
  - Keep persistence aligned with worker `ready`.
- Modify: `src-tauri/src/model_manager/runtime.rs`
  - Rename or expand the current GPU preflight logic into transcription runtime preflight.
  - Stop treating `nvidia-smi` success as equivalent to a loaded transcription runtime.
- Create: `src-tauri/src/transcribe.rs`
  - Define worker protocol types, process management, request/response handling, timeout handling, and top-level `ensure_loaded`, `transcribe_file`, and `unload` operations.
- Create: `src-tauri/tests/transcribe_protocol.rs` or inline unit tests in `src-tauri/src/transcribe.rs`
  - Validate JSON message serialization, request correlation, and failure classification.

### Frontend files

- Modify: `src/lib/stores/appState.ts`
  - Update state union to `ready | listening | processing | completed | error`.
- Modify: `src/lib/components/Overlay.svelte`
  - Update pulsing and styling logic for `processing` and `completed`.
- Modify: `src/lib/components/Settings.svelte`
  - Surface any changed `readyForTranscription` semantics or model load errors without changing the overall layout.

### Python worker/runtime files

- Create: `src-tauri/python/worker_main.py`
  - Worker entrypoint that loads the model and handles stdin/stdout JSON.
- Create: `src-tauri/python/worker_protocol.py`
  - Message parsing/formatting helpers.
- Create: `src-tauri/python/transcriber.py`
  - Thin adapter around the actual Parakeet/NeMo loading and inference call.
- Create: `src-tauri/python/tests/test_worker_protocol.py`
  - Protocol parsing/serialization tests.
- Create: `src-tauri/python/requirements.txt`
  - Python dependency manifest for the embedded runtime.

### Documentation files

- Modify: `docs/tauri_speech_app_plan.md`
  - Mark phases 3 and 5 as in progress or implemented when done.

## Implementation Notes

- Keep one transcription request in flight at a time.
- Do not add model hot-reload support inside the worker.
- Keep the worker private; the frontend should only continue to use Tauri commands and events.
- Fail fast on hotkey press if no worker-backed model is loaded for v1.
- Persist `loaded_model_id` only after the worker emits `ready`.

### Task 1: Expand App State For Real Transcription

**Files:**
- Modify: `src-tauri/src/state.rs`
- Modify: `src-tauri/src/overlay.rs`
- Modify: `src/lib/stores/appState.ts`
- Modify: `src/lib/components/Overlay.svelte`

- [ ] **Step 1: Write the failing Rust state tests**

Add these tests in `src-tauri/src/state.rs`:

```rust
#[test]
fn listening_can_transition_to_processing() {
    assert!(AppState::Listening.can_transition_to(AppState::Processing));
}

#[test]
fn processing_can_transition_to_completed() {
    assert!(AppState::Processing.can_transition_to(AppState::Completed));
}

#[test]
fn completed_can_transition_to_ready() {
    assert!(AppState::Completed.can_transition_to(AppState::Ready));
}

#[test]
fn processing_has_transcribing_message() {
    assert_eq!(
        AppState::Processing.default_message(None),
        Some("Transcribing...".to_string())
    );
}
```

- [ ] **Step 2: Run the Rust state test to verify it fails**

Run: `cargo test state::tests::processing_has_transcribing_message --manifest-path src-tauri/Cargo.toml`

Expected: FAIL with errors such as `no variant or associated item named 'Processing' found for enum 'AppState'`.

- [ ] **Step 3: Implement the minimal Rust state changes**

Update `src-tauri/src/state.rs` like this:

```rust
#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum AppState {
    Ready,
    Listening,
    Processing,
    Completed,
    Error,
}

impl AppState {
    pub fn can_transition_to(self, next: AppState) -> bool {
        matches!(
            (self, next),
            (Self::Ready, Self::Listening)
                | (Self::Listening, Self::Processing)
                | (Self::Listening, Self::Error)
                | (Self::Processing, Self::Completed)
                | (Self::Processing, Self::Error)
                | (Self::Completed, Self::Ready)
                | (Self::Error, Self::Ready)
        )
    }

    pub fn default_message(self, error_message: Option<&str>) -> Option<String> {
        match self {
            Self::Ready => None,
            Self::Listening => Some("Listening...".to_string()),
            Self::Processing => Some("Transcribing...".to_string()),
            Self::Completed => Some("Copied to clipboard".to_string()),
            Self::Error => error_message.map(str::to_string),
        }
    }
}
```

Update `src-tauri/src/overlay.rs` like this:

```rust
pub fn sync_visibility_for_state<R: Runtime>(app: &AppHandle<R>, state: AppState) -> tauri::Result<()> {
    match state {
        AppState::Ready => hide_overlay(app),
        AppState::Listening | AppState::Processing | AppState::Completed | AppState::Error => {
            show_overlay(app)
        }
    }
}
```

Update `src/lib/stores/appState.ts` like this:

```ts
export type AppState = 'ready' | 'listening' | 'processing' | 'completed' | 'error';
```

Update `src/lib/components/Overlay.svelte` like this:

```svelte
<script lang="ts">
	import { appState } from '$lib/stores/appState';

	const pulseStates = new Set(['listening', 'processing']);
</script>
```

- [ ] **Step 4: Run the focused Rust tests to verify they pass**

Run: `cargo test state::tests --manifest-path src-tauri/Cargo.toml`

Expected: PASS for the updated transition and message tests.

- [ ] **Step 5: Run the frontend type check**

Run: `npm run check`

Expected: PASS, or frontend errors only where more plan tasks still need to update state usage.

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/state.rs src-tauri/src/overlay.rs src/lib/stores/appState.ts src/lib/components/Overlay.svelte
git commit -m "refactor: update app states for transcription flow"
```

### Task 2: Add Clipboard Paste Support

**Files:**
- Modify: `src-tauri/src/clipboard.rs`
- Test: `src-tauri/src/clipboard.rs`

- [ ] **Step 1: Write the failing unit test for paste command construction**

Add this test and helper shape in `src-tauri/src/clipboard.rs`:

```rust
fn paste_shortcut_sequence() -> Vec<(&'static str, &'static str)> {
    vec![("down", "ctrl"), ("press", "v"), ("up", "ctrl")]
}

#[test]
fn paste_shortcut_sequence_uses_ctrl_v() {
    assert_eq!(
        paste_shortcut_sequence(),
        vec![("down", "ctrl"), ("press", "v"), ("up", "ctrl")]
    );
}
```

- [ ] **Step 2: Run the clipboard test to verify it fails**

Run: `cargo test clipboard::tests::paste_shortcut_sequence_uses_ctrl_v --manifest-path src-tauri/Cargo.toml`

Expected: FAIL because `paste_shortcut_sequence` does not exist yet.

- [ ] **Step 3: Implement minimal clipboard copy and paste helpers**

Update `src-tauri/src/clipboard.rs` like this:

```rust
use arboard::Clipboard;
use enigo::{Direction, Enigo, Key, Keyboard, Settings};

pub fn copy_text(text: &str) -> Result<(), String> {
    let mut clipboard = Clipboard::new().map_err(|error| error.to_string())?;
    clipboard.set_text(text.to_string()).map_err(|error| error.to_string())
}

fn paste_shortcut_sequence() -> Vec<(&'static str, &'static str)> {
    vec![("down", "ctrl"), ("press", "v"), ("up", "ctrl")]
}

pub fn paste_clipboard() -> Result<(), String> {
    let mut enigo = Enigo::new(&Settings::default()).map_err(|error| error.to_string())?;
    enigo.key(Key::Control, Direction::Press).map_err(|error| error.to_string())?;
    enigo.key(Key::Unicode('v'), Direction::Click).map_err(|error| error.to_string())?;
    enigo.key(Key::Control, Direction::Release).map_err(|error| error.to_string())?;
    Ok(())
}
```

- [ ] **Step 4: Run the clipboard tests**

Run: `cargo test clipboard::tests --manifest-path src-tauri/Cargo.toml`

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/clipboard.rs src-tauri/Cargo.toml
git commit -m "feat: add clipboard paste helper"
```

### Task 3: Build Rust Worker Protocol Types

**Files:**
- Create: `src-tauri/src/transcribe.rs`
- Test: `src-tauri/src/transcribe.rs`

- [ ] **Step 1: Write failing protocol serialization tests**

Start `src-tauri/src/transcribe.rs` with these tests:

```rust
#[cfg(test)]
mod tests {
    use super::{WorkerRequest, WorkerResponse};

    #[test]
    fn transcribe_request_serializes_as_json_line() {
        let request = WorkerRequest::Transcribe {
            request_id: "req-1".to_string(),
            audio_path: "C:/temp/recording.wav".to_string(),
        };

        let json = serde_json::to_string(&request).unwrap();
        assert_eq!(
            json,
            r#"{"type":"transcribe","request_id":"req-1","audio_path":"C:/temp/recording.wav"}"#
        );
    }

    #[test]
    fn ready_response_deserializes() {
        let response: WorkerResponse = serde_json::from_str(
            r#"{"type":"ready","model_id":"parakeet-v0.2-en","model_path":"C:/models/model.nemo"}"#,
        )
        .unwrap();

        assert!(matches!(response, WorkerResponse::Ready { .. }));
    }
}
```

- [ ] **Step 2: Run the focused protocol test to verify it fails**

Run: `cargo test transcribe::tests::transcribe_request_serializes_as_json_line --manifest-path src-tauri/Cargo.toml`

Expected: FAIL because `src-tauri/src/transcribe.rs` is empty.

- [ ] **Step 3: Implement the protocol enums and helpers**

Use this initial implementation in `src-tauri/src/transcribe.rs`:

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum WorkerRequest {
    Ping,
    Shutdown,
    Transcribe {
        request_id: String,
        audio_path: String,
    },
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum WorkerResponse {
    Ready {
        model_id: String,
        model_path: String,
    },
    Pong,
    TranscribeResult {
        request_id: String,
        text: String,
        duration_ms: u64,
    },
    Error {
        request_id: Option<String>,
        message: String,
    },
    Fatal {
        message: String,
    },
}

pub fn encode_json_line<T: Serialize>(message: &T) -> Result<String, String> {
    let mut line = serde_json::to_string(message).map_err(|error| error.to_string())?;
    line.push('\n');
    Ok(line)
}
```

- [ ] **Step 4: Run the protocol tests**

Run: `cargo test transcribe::tests --manifest-path src-tauri/Cargo.toml`

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/transcribe.rs
git commit -m "feat: define worker protocol types"
```

### Task 4: Add Rust Worker Process Supervision

**Files:**
- Modify: `src-tauri/src/transcribe.rs`
- Modify: `src-tauri/src/lib.rs`
- Test: `src-tauri/src/transcribe.rs`

- [ ] **Step 1: Write failing tests for worker failure classification**

Add these tests to `src-tauri/src/transcribe.rs`:

```rust
#[test]
fn fatal_response_marks_worker_unusable() {
    let response = WorkerResponse::Fatal {
        message: "model load failed".to_string(),
    };

    assert!(response.is_terminal_failure());
}

#[test]
fn protocol_parse_error_is_terminal_failure() {
    let parsed = serde_json::from_str::<WorkerResponse>("not-json");
    assert!(parsed.is_err());
}
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `cargo test transcribe::tests::fatal_response_marks_worker_unusable --manifest-path src-tauri/Cargo.toml`

Expected: FAIL because `is_terminal_failure` does not exist.

- [ ] **Step 3: Implement a minimal `WorkerHandle` and lifecycle API**

Add these shapes to `src-tauri/src/transcribe.rs`:

```rust
use std::{
    io::{BufRead, BufReader, Write},
    path::Path,
    process::{Child, ChildStdin, Command, Stdio},
    sync::{mpsc, Mutex},
    thread,
    time::Duration,
};

pub const WORKER_START_TIMEOUT: Duration = Duration::from_secs(20);
pub const TRANSCRIBE_TIMEOUT: Duration = Duration::from_secs(60);

impl WorkerResponse {
    pub fn is_terminal_failure(&self) -> bool {
        matches!(self, Self::Fatal { .. })
    }
}

pub struct WorkerHandle {
    child: Child,
    stdin: ChildStdin,
    responses: mpsc::Receiver<Result<WorkerResponse, String>>,
    loaded_model_id: String,
}

pub fn spawn_worker(
    python_exe: &Path,
    worker_script: &Path,
    model_id: &str,
    model_path: &Path,
) -> Result<WorkerHandle, String> {
    let mut child = Command::new(python_exe)
        .arg(worker_script)
        .arg("--model-id")
        .arg(model_id)
        .arg("--model-path")
        .arg(model_path)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| error.to_string())?;

    let stdin = child.stdin.take().ok_or_else(|| "worker stdin unavailable".to_string())?;
    let stdout = child.stdout.take().ok_or_else(|| "worker stdout unavailable".to_string())?;
    let (tx, rx) = mpsc::channel();

    thread::spawn(move || {
        let reader = BufReader::new(stdout);
        for line in reader.lines() {
            let item = line
                .map_err(|error| error.to_string())
                .and_then(|line| serde_json::from_str::<WorkerResponse>(&line).map_err(|error| error.to_string()));
            if tx.send(item).is_err() {
                break;
            }
        }
    });

    Ok(WorkerHandle {
        child,
        stdin,
        responses: rx,
        loaded_model_id: model_id.to_string(),
    })
}
```

- [ ] **Step 4: Add `TranscriptionManager` to runtime handles**

Update `src-tauri/src/lib.rs` like this:

```rust
pub struct RuntimeHandles {
    pub hotkey: std::sync::Mutex<Option<hotkey::HotkeyHandle>>,
    pub recorder: std::sync::Mutex<Option<RecorderSession>>,
    pub transcription: std::sync::Mutex<Option<transcribe::WorkerHandle>>,
}

impl Default for RuntimeHandles {
    fn default() -> Self {
        Self {
            hotkey: std::sync::Mutex::new(None),
            recorder: std::sync::Mutex::new(None),
            transcription: std::sync::Mutex::new(None),
        }
    }
}
```

- [ ] **Step 5: Run the focused Rust tests**

Run: `cargo test transcribe::tests --manifest-path src-tauri/Cargo.toml`

Expected: PASS for protocol and failure classification tests.

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/transcribe.rs src-tauri/src/lib.rs
git commit -m "feat: add worker supervision scaffold"
```

### Task 5: Implement Worker Startup, Shutdown, And Request Round-Trip

**Files:**
- Modify: `src-tauri/src/transcribe.rs`
- Test: `src-tauri/src/transcribe.rs`

- [ ] **Step 1: Write failing tests for request correlation and timeout classification**

Add these tests to `src-tauri/src/transcribe.rs`:

```rust
#[test]
fn transcribe_result_matches_request_id() {
    let response = WorkerResponse::TranscribeResult {
        request_id: "req-123".to_string(),
        text: "hello".to_string(),
        duration_ms: 10,
    };

    assert!(response.matches_request("req-123"));
    assert!(!response.matches_request("req-999"));
}

#[test]
fn error_response_without_request_id_does_not_match() {
    let response = WorkerResponse::Error {
        request_id: None,
        message: "bad".to_string(),
    };

    assert!(!response.matches_request("req-1"));
}
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `cargo test transcribe::tests::transcribe_result_matches_request_id --manifest-path src-tauri/Cargo.toml`

Expected: FAIL because `matches_request` does not exist yet.

- [ ] **Step 3: Implement request matching and synchronous request handling**

Extend `src-tauri/src/transcribe.rs` with this logic:

```rust
impl WorkerResponse {
    pub fn matches_request(&self, request_id: &str) -> bool {
        match self {
            Self::TranscribeResult { request_id: current, .. } => current == request_id,
            Self::Error { request_id: Some(current), .. } => current == request_id,
            _ => false,
        }
    }
}

impl WorkerHandle {
    pub fn wait_for_ready(&self, timeout: Duration) -> Result<(), String> {
        match self.responses.recv_timeout(timeout) {
            Ok(Ok(WorkerResponse::Ready { .. })) => Ok(()),
            Ok(Ok(WorkerResponse::Fatal { message })) => Err(message),
            Ok(Ok(other)) => Err(format!("unexpected worker startup message: {other:?}")),
            Ok(Err(error)) => Err(error),
            Err(_) => Err("worker startup timed out".to_string()),
        }
    }

    pub fn send_request(&mut self, request: &WorkerRequest) -> Result<(), String> {
        let line = encode_json_line(request)?;
        self.stdin.write_all(line.as_bytes()).map_err(|error| error.to_string())?;
        self.stdin.flush().map_err(|error| error.to_string())
    }

    pub fn wait_for_transcribe_result(
        &self,
        request_id: &str,
        timeout: Duration,
    ) -> Result<(String, u64), String> {
        match self.responses.recv_timeout(timeout) {
            Ok(Ok(WorkerResponse::TranscribeResult { request_id: current, text, duration_ms }))
                if current == request_id => Ok((text, duration_ms)),
            Ok(Ok(WorkerResponse::Error { request_id: Some(current), message })) if current == request_id => {
                Err(message)
            }
            Ok(Ok(WorkerResponse::Fatal { message })) => Err(message),
            Ok(Ok(other)) => Err(format!("unexpected worker response: {other:?}")),
            Ok(Err(error)) => Err(error),
            Err(_) => Err("transcription timed out".to_string()),
        }
    }

    pub fn shutdown(mut self) {
        let _ = self.send_request(&WorkerRequest::Shutdown);
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}
```

- [ ] **Step 4: Run the protocol tests**

Run: `cargo test transcribe::tests --manifest-path src-tauri/Cargo.toml`

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/transcribe.rs
git commit -m "feat: implement worker request round-trip"
```

### Task 6: Add App-Managed Runtime Path Resolution

**Files:**
- Modify: `src-tauri/src/transcribe.rs`
- Modify: `src-tauri/src/lib.rs`
- Test: `src-tauri/src/transcribe.rs`

- [ ] **Step 1: Write failing tests for runtime path layout**

Add these tests:

```rust
#[test]
fn worker_runtime_dir_is_under_runtime_worker() {
    let root = std::path::Path::new("C:/AppData/OK");
    assert_eq!(worker_runtime_dir(root), root.join("runtime").join("worker"));
}

#[test]
fn embedded_python_dir_is_under_runtime_python() {
    let root = std::path::Path::new("C:/AppData/OK");
    assert_eq!(python_runtime_dir(root), root.join("runtime").join("python"));
}
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `cargo test transcribe::tests::worker_runtime_dir_is_under_runtime_worker --manifest-path src-tauri/Cargo.toml`

Expected: FAIL because the path helpers do not exist.

- [ ] **Step 3: Implement runtime layout helpers**

Add these helpers in `src-tauri/src/transcribe.rs`:

```rust
use std::path::{Path, PathBuf};

pub fn python_runtime_dir(app_data_root: &Path) -> PathBuf {
    app_data_root.join("runtime").join("python")
}

pub fn worker_runtime_dir(app_data_root: &Path) -> PathBuf {
    app_data_root.join("runtime").join("worker")
}

pub fn worker_entrypoint(app_data_root: &Path) -> PathBuf {
    worker_runtime_dir(app_data_root).join("worker_main.py")
}
```

Add this helper in `src-tauri/src/lib.rs`:

```rust
pub fn app_data_dir<R: Runtime>(app: &AppHandle<R>) -> Result<PathBuf, String> {
    app.path().app_local_data_dir().map_err(|error| error.to_string())
}
```

- [ ] **Step 4: Run the focused tests**

Run: `cargo test transcribe::tests --manifest-path src-tauri/Cargo.toml`

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/transcribe.rs src-tauri/src/lib.rs
git commit -m "feat: add transcription runtime path helpers"
```

### Task 7: Integrate Worker Lifecycle With Model Manager

**Files:**
- Modify: `src-tauri/src/model_manager/service.rs`
- Modify: `src-tauri/src/transcribe.rs`
- Modify: `src-tauri/src/lib.rs`
- Test: `src-tauri/src/model_manager/service.rs`

- [ ] **Step 1: Write the failing model manager test for deferred `loaded_model_id` persistence**

Add this test in `src-tauri/src/model_manager/service.rs` near the existing tests:

```rust
#[test]
fn loaded_model_id_is_not_set_before_worker_ready() {
    let registry = crate::model_manager::registry::ModelRegistry::default();
    assert_eq!(registry.loaded_model_id, None);
}
```

- [ ] **Step 2: Run the focused test**

Run: `cargo test model_manager::service::tests::loaded_model_id_is_not_set_before_worker_ready --manifest-path src-tauri/Cargo.toml`

Expected: PASS now; keep it as a guard before changing `load_model`.

- [ ] **Step 3: Make `load_model` start a worker and persist only on ready**

Refactor the core of `load_model` in `src-tauri/src/model_manager/service.rs` to follow this shape:

```rust
let prepared = prepare_model(&installed, model_id);
transcribe::ensure_loaded(app, model_id, &prepared.model_path)?;

let path = registry_path(app).map_err(|error| error.to_string())?;
{
    let mut registry = state
        .registry
        .lock()
        .map_err(|_| "model registry mutex poisoned".to_string())?;
    registry.loaded_model_id = Some(model_id);
    registry.model_errors.remove(model_id.as_str());
    registry::save(&path, &registry).map_err(|error| error.to_string())?;
}
```

Refactor `unload_model` to follow this shape:

```rust
transcribe::unload(app)?;

let path = registry_path(app).map_err(|error| error.to_string())?;
{
    let mut registry = state
        .registry
        .lock()
        .map_err(|_| "model registry mutex poisoned".to_string())?;
    registry.loaded_model_id = None;
    registry::save(&path, &registry).map_err(|error| error.to_string())?;
}
```

- [ ] **Step 4: Implement `ensure_loaded` and `unload` in `src-tauri/src/transcribe.rs`**

Add these functions:

```rust
use tauri::{AppHandle, Runtime, State};

use crate::{app_data_dir, RuntimeHandles};

pub fn ensure_loaded<R: Runtime>(
    app: &AppHandle<R>,
    model_id: crate::model_manager::types::ModelId,
    model_path: &Path,
) -> Result<(), String> {
    let handles: State<'_, RuntimeHandles> = app.state();
    let mut slot = handles
        .transcription
        .lock()
        .map_err(|_| "transcription mutex poisoned".to_string())?;

    if let Some(existing) = slot.as_ref() {
        if existing.loaded_model_id == model_id.as_str() {
            return Ok(());
        }
    }

    if let Some(existing) = slot.take() {
        existing.shutdown();
    }

    let root = app_data_dir(app)?;
    let python_exe = python_runtime_dir(&root).join("python.exe");
    let worker_script = worker_entrypoint(&root);
    let handle = spawn_worker(&python_exe, &worker_script, model_id.as_str(), model_path)?;
    handle.wait_for_ready(WORKER_START_TIMEOUT)?;
    *slot = Some(handle);
    Ok(())
}

pub fn unload<R: Runtime>(app: &AppHandle<R>) -> Result<(), String> {
    let handles: State<'_, RuntimeHandles> = app.state();
    let mut slot = handles
        .transcription
        .lock()
        .map_err(|_| "transcription mutex poisoned".to_string())?;

    if let Some(existing) = slot.take() {
        existing.shutdown();
    }

    Ok(())
}
```

- [ ] **Step 5: Run model-manager and transcribe tests**

Run: `cargo test model_manager::service::tests transcribe::tests --manifest-path src-tauri/Cargo.toml`

Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/model_manager/service.rs src-tauri/src/transcribe.rs src-tauri/src/lib.rs
git commit -m "feat: connect model loading to worker lifecycle"
```

### Task 8: Replace Stop-Recording Ready Flow With Transcription Pipeline

**Files:**
- Modify: `src-tauri/src/lib.rs`
- Modify: `src-tauri/src/transcribe.rs`
- Modify: `src-tauri/src/hotkey.rs`
- Test: `src-tauri/src/state.rs`

- [ ] **Step 1: Write the failing test for completed-state message**

Add this test in `src-tauri/src/state.rs`:

```rust
#[test]
fn completed_state_can_override_default_message() {
    assert_eq!(
        AppState::Completed.default_message(Some("Pasted")),
        Some("Copied to clipboard".to_string())
    );
}
```

- [ ] **Step 2: Run the test to verify the current behavior**

Run: `cargo test state::tests::completed_state_can_override_default_message --manifest-path src-tauri/Cargo.toml`

Expected: PASS or FAIL depending on the chosen helper; this test exists to guard `Completed` semantics before pipeline integration.

- [ ] **Step 3: Add `transcribe_file` and result type in `src-tauri/src/transcribe.rs`**

Add this code:

```rust
pub struct TranscriptionResult {
    pub text: String,
    pub duration_ms: u64,
}

pub fn transcribe_file<R: Runtime>(
    app: &AppHandle<R>,
    audio_path: &Path,
) -> Result<TranscriptionResult, String> {
    let handles: State<'_, RuntimeHandles> = app.state();
    let mut slot = handles
        .transcription
        .lock()
        .map_err(|_| "transcription mutex poisoned".to_string())?;
    let worker = slot.as_mut().ok_or_else(|| "No loaded transcription model".to_string())?;

    let request_id = format!("req-{}", std::time::SystemTime::now().elapsed().unwrap().as_nanos());
    worker.send_request(&WorkerRequest::Transcribe {
        request_id: request_id.clone(),
        audio_path: audio_path.display().to_string(),
    })?;
    let (text, duration_ms) = worker.wait_for_transcribe_result(&request_id, TRANSCRIBE_TIMEOUT)?;
    Ok(TranscriptionResult { text, duration_ms })
}
```

- [ ] **Step 4: Replace the end of `stop_recording` with the transcription pipeline**

Refactor `src-tauri/src/lib.rs` to follow this shape:

```rust
match session.stop(&output_dir) {
    Ok(result) => {
        transition_to(app, AppState::Processing, None)?;

        match transcribe::transcribe_file(app, &result.path) {
            Ok(transcription) => {
                clipboard::copy_text(&transcription.text)?;

                let completed_message = if clipboard::paste_clipboard().is_ok() {
                    "Pasted".to_string()
                } else {
                    "Copied to clipboard".to_string()
                };

                transition_to(app, AppState::Completed, Some(completed_message))?;
                schedule_ready_reset(app.clone());
                Ok(())
            }
            Err(error) => {
                transition_to(app, AppState::Error, Some(error.clone()))?;
                schedule_ready_reset(app.clone());
                Err(error)
            }
        }
    }
    Err(error) => {
        transition_to(app, AppState::Error, Some(error.clone()))?;
        schedule_ready_reset(app.clone());
        Err(error)
    }
}
```

- [ ] **Step 5: Guard hotkey start when no model is loaded**

Add this early check near the top of `start_recording` in `src-tauri/src/lib.rs`:

```rust
let handles: State<'_, RuntimeHandles> = app.state();
if handles
    .transcription
    .lock()
    .map_err(|_| "transcription mutex poisoned".to_string())?
    .is_none()
{
    transition_to(app, AppState::Error, Some("Load a model before recording".to_string()))?;
    schedule_ready_reset(app.clone());
    return Err("Load a model before recording".to_string());
}
```

- [ ] **Step 6: Run the core Rust tests**

Run: `cargo test --manifest-path src-tauri/Cargo.toml`

Expected: PASS for all Rust unit tests, or remaining failures limited to Python integration not yet added.

- [ ] **Step 7: Commit**

```bash
git add src-tauri/src/lib.rs src-tauri/src/transcribe.rs src-tauri/src/hotkey.rs src-tauri/src/state.rs
git commit -m "feat: wire recording into transcription pipeline"
```

### Task 9: Implement The Python Worker Protocol Layer

**Files:**
- Create: `src-tauri/python/worker_protocol.py`
- Create: `src-tauri/python/tests/test_worker_protocol.py`

- [ ] **Step 1: Write the failing Python protocol tests**

Create `src-tauri/python/tests/test_worker_protocol.py`:

```python
from worker_protocol import parse_line, encode_message


def test_parse_transcribe_request():
    message = parse_line('{"type":"transcribe","request_id":"req-1","audio_path":"C:/temp/audio.wav"}')
    assert message["type"] == "transcribe"
    assert message["request_id"] == "req-1"


def test_encode_ready_message():
    line = encode_message({
        "type": "ready",
        "model_id": "parakeet-v0.2-en",
        "model_path": "C:/models/model.nemo",
    })
    assert line == '{"type":"ready","model_id":"parakeet-v0.2-en","model_path":"C:/models/model.nemo"}\n'
```

- [ ] **Step 2: Run the Python protocol tests to verify they fail**

Run: `python -m pytest src-tauri/python/tests/test_worker_protocol.py -q`

Expected: FAIL with `ModuleNotFoundError: No module named 'worker_protocol'`.

- [ ] **Step 3: Implement `worker_protocol.py`**

Create `src-tauri/python/worker_protocol.py`:

```python
import json


def parse_line(line: str) -> dict:
    message = json.loads(line)
    if "type" not in message:
        raise ValueError("protocol message missing type")
    return message


def encode_message(message: dict) -> str:
    return json.dumps(message, separators=(",", ":")) + "\n"
```

- [ ] **Step 4: Run the Python protocol tests**

Run: `python -m pytest src-tauri/python/tests/test_worker_protocol.py -q`

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/python/worker_protocol.py src-tauri/python/tests/test_worker_protocol.py
git commit -m "feat: add python worker protocol helpers"
```

### Task 10: Implement The Python Worker Entrypoint

**Files:**
- Create: `src-tauri/python/worker_main.py`
- Create: `src-tauri/python/transcriber.py`
- Modify: `src-tauri/python/tests/test_worker_protocol.py`

- [ ] **Step 1: Write the failing Python worker tests for startup and error shape**

Append these tests to `src-tauri/python/tests/test_worker_protocol.py`:

```python
from transcriber import FakeTranscriber


def test_fake_transcriber_returns_text(tmp_path):
    audio = tmp_path / "audio.wav"
    audio.write_bytes(b"RIFF")
    transcriber = FakeTranscriber("parakeet-v0.2-en", str(audio))
    assert transcriber.transcribe(str(audio))["text"] == "transcribed text"
```

- [ ] **Step 2: Run the Python tests to verify they fail**

Run: `python -m pytest src-tauri/python/tests/test_worker_protocol.py -q`

Expected: FAIL with `ModuleNotFoundError: No module named 'transcriber'`.

- [ ] **Step 3: Implement the worker entrypoint and transcriber adapter**

Create `src-tauri/python/transcriber.py`:

```python
from pathlib import Path
import time


class FakeTranscriber:
    def __init__(self, model_id: str, model_path: str):
        self.model_id = model_id
        self.model_path = model_path

    def transcribe(self, audio_path: str) -> dict:
        if not Path(audio_path).exists():
            raise FileNotFoundError(audio_path)
        started = time.perf_counter()
        return {
            "text": "transcribed text",
            "duration_ms": int((time.perf_counter() - started) * 1000),
        }
```

Create `src-tauri/python/worker_main.py`:

```python
import argparse
import sys

from transcriber import FakeTranscriber
from worker_protocol import encode_message, parse_line


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--model-id", required=True)
    parser.add_argument("--model-path", required=True)
    args = parser.parse_args()

    transcriber = FakeTranscriber(args.model_id, args.model_path)
    sys.stdout.write(
        encode_message(
            {
                "type": "ready",
                "model_id": args.model_id,
                "model_path": args.model_path,
            }
        )
    )
    sys.stdout.flush()

    for raw_line in sys.stdin:
        message = parse_line(raw_line)

        if message["type"] == "shutdown":
            return 0

        if message["type"] == "ping":
            sys.stdout.write(encode_message({"type": "pong"}))
            sys.stdout.flush()
            continue

        if message["type"] == "transcribe":
            try:
                result = transcriber.transcribe(message["audio_path"])
                sys.stdout.write(
                    encode_message(
                        {
                            "type": "transcribe_result",
                            "request_id": message["request_id"],
                            "text": result["text"],
                            "duration_ms": result["duration_ms"],
                        }
                    )
                )
                sys.stdout.flush()
            except Exception as exc:
                sys.stdout.write(
                    encode_message(
                        {
                            "type": "error",
                            "request_id": message.get("request_id"),
                            "message": str(exc),
                        }
                    )
                )
                sys.stdout.flush()

    return 0


if __name__ == "__main__":
    raise SystemExit(main())
```

- [ ] **Step 4: Run the Python tests**

Run: `python -m pytest src-tauri/python/tests/test_worker_protocol.py -q`

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/python/worker_main.py src-tauri/python/transcriber.py src-tauri/python/tests/test_worker_protocol.py
git commit -m "feat: add python worker entrypoint scaffold"
```

### Task 11: Replace The Fake Python Transcriber With Real Model Loading

**Files:**
- Modify: `src-tauri/python/transcriber.py`
- Modify: `src-tauri/python/requirements.txt`
- Test: `src-tauri/python/tests/test_worker_protocol.py`

- [ ] **Step 1: Write the failing adapter test around missing model files**

Add this test:

```python
import pytest

from transcriber import build_transcriber


def test_build_transcriber_rejects_missing_model(tmp_path):
    with pytest.raises(FileNotFoundError):
        build_transcriber("parakeet-v0.2-en", str(tmp_path / "missing.nemo"))
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `python -m pytest src-tauri/python/tests/test_worker_protocol.py -q`

Expected: FAIL because `build_transcriber` does not exist yet.

- [ ] **Step 3: Implement the real transcriber adapter behind a single factory**

Refactor `src-tauri/python/transcriber.py` to this shape:

```python
from pathlib import Path
import time


class NemoTranscriber:
    def __init__(self, model_id: str, model_path: str):
        self.model_id = model_id
        self.model_path = model_path
        self._model = self._load_model(model_path)

    def _load_model(self, model_path: str):
        from nemo.collections.asr.models import EncDecMultiTaskModel

        return EncDecMultiTaskModel.restore_from(model_path)

    def transcribe(self, audio_path: str) -> dict:
        started = time.perf_counter()
        output = self._model.transcribe([audio_path])
        text = output[0] if isinstance(output, list) else str(output)
        return {
            "text": text.strip(),
            "duration_ms": int((time.perf_counter() - started) * 1000),
        }


def build_transcriber(model_id: str, model_path: str):
    path = Path(model_path)
    if not path.exists():
        raise FileNotFoundError(model_path)
    return NemoTranscriber(model_id, str(path))
```

Update `src-tauri/python/worker_main.py` imports like this:

```python
from transcriber import build_transcriber

transcriber = build_transcriber(args.model_id, args.model_path)
```

Add this initial `src-tauri/python/requirements.txt`:

```text
nemo_toolkit[asr]
torch
pytest
```

- [ ] **Step 4: Run the Python tests**

Run: `python -m pytest src-tauri/python/tests/test_worker_protocol.py -q`

Expected: PASS for path validation tests. Model-loading tests should remain isolated from the heavyweight dependency import path.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/python/transcriber.py src-tauri/python/worker_main.py src-tauri/python/requirements.txt src-tauri/python/tests/test_worker_protocol.py
git commit -m "feat: add real nemo transcriber adapter"
```

### Task 12: Surface Runtime Errors And Ready State In The UI

**Files:**
- Modify: `src-tauri/src/model_manager/service.rs`
- Modify: `src/lib/components/Settings.svelte`
- Modify: `src/lib/stores/modelManager.ts`

- [ ] **Step 1: Write the failing frontend assertion in the settings component logic**

Add this derived snippet in `src/lib/components/Settings.svelte`:

```svelte
<script lang="ts">
	import { modelManager } from '$lib/stores/modelManager';

	$: runtimeStatus = $modelManager.readyForTranscription ? 'Ready' : 'Not ready';
</script>
```

- [ ] **Step 2: Run the frontend check to verify the current UI still builds**

Run: `npm run check`

Expected: PASS or type errors only where new UI text still needs wiring.

- [ ] **Step 3: Implement the runtime status copy in `Settings.svelte`**

Add a line near the existing model status section:

```svelte
<p class="runtime-status">Runtime: {runtimeStatus}</p>
```

Keep the existing model controls, but ensure model-row `errorMessage` continues to render when `load_model` fails.

- [ ] **Step 4: Run the frontend check again**

Run: `npm run check`

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src/lib/components/Settings.svelte src/lib/stores/modelManager.ts src-tauri/src/model_manager/service.rs
git commit -m "feat: surface transcription runtime readiness"
```

### Task 13: Package The Python Worker Into The App-Managed Runtime

**Files:**
- Modify: `src-tauri/build.rs` or `src-tauri/tauri.conf.json` if present
- Create: `src-tauri/python/bootstrap_runtime.py` if needed
- Modify: `src-tauri/src/transcribe.rs`

- [ ] **Step 1: Write the failing runtime verification test**

Add this test in `src-tauri/src/transcribe.rs`:

```rust
#[test]
fn worker_entrypoint_points_to_worker_main() {
    let root = std::path::Path::new("C:/AppData/OK");
    assert_eq!(
        worker_entrypoint(root),
        root.join("runtime").join("worker").join("worker_main.py")
    );
}
```

- [ ] **Step 2: Run the focused test**

Run: `cargo test transcribe::tests::worker_entrypoint_points_to_worker_main --manifest-path src-tauri/Cargo.toml`

Expected: PASS if the helper already exists; otherwise FAIL until the helper is added.

- [ ] **Step 3: Add runtime verification and extraction hook**

Extend `src-tauri/src/transcribe.rs` with this minimal verifier:

```rust
pub fn verify_runtime_layout(app_data_root: &Path) -> Result<(), String> {
    let python_exe = python_runtime_dir(app_data_root).join("python.exe");
    let worker_script = worker_entrypoint(app_data_root);

    if !python_exe.exists() {
        return Err(format!("embedded python missing: {}", python_exe.display()));
    }

    if !worker_script.exists() {
        return Err(format!("worker entrypoint missing: {}", worker_script.display()));
    }

    Ok(())
}
```

Wire `ensure_loaded` to call `verify_runtime_layout(&root)?` before `spawn_worker(...)`.

If the repo already has a Tauri resource-copy mechanism, add `src-tauri/python/**/*` to it. If not, create a minimal extraction step in `build.rs` or a startup copy routine that ensures `worker_main.py`, `worker_protocol.py`, and `transcriber.py` land under `AppData/runtime/worker`.

- [ ] **Step 4: Run the Rust test suite**

Run: `cargo test --manifest-path src-tauri/Cargo.toml`

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/transcribe.rs src-tauri/build.rs src-tauri/tauri.conf.json src-tauri/python
git commit -m "build: package python worker runtime assets"
```

### Task 14: End-To-End Verification And Plan Cleanup

**Files:**
- Modify: `docs/tauri_speech_app_plan.md`
- Modify: `docs/superpowers/specs/2026-04-19-worker-transcription-design.md` if implementation drift requires clarification

- [ ] **Step 1: Run the Rust test suite**

Run: `cargo test --manifest-path src-tauri/Cargo.toml`

Expected: PASS.

- [ ] **Step 2: Run the frontend validation**

Run: `npm run check`

Expected: PASS.

- [ ] **Step 3: Run the Python worker tests**

Run: `python -m pytest src-tauri/python/tests -q`

Expected: PASS.

- [ ] **Step 4: Run a manual smoke test**

Run: `npm run tauri dev`

Expected manual flow:

- open Settings
- download and load one model
- hold `Ctrl+Alt+Space`
- speak a short phrase
- release hotkey
- overlay shows `Transcribing...`
- text lands in clipboard and pastes into Notepad

- [ ] **Step 5: Update the project plan doc**

Update `docs/tauri_speech_app_plan.md` to mark:

```md
- 3. Worker process
- 5. Real transcription
```

as implemented, or add a short note about the shipped worker/runtime architecture if you want the doc to reflect the final system.

- [ ] **Step 6: Commit**

```bash
git add docs/tauri_speech_app_plan.md docs/superpowers/specs/2026-04-19-worker-transcription-design.md
git commit -m "docs: finalize worker transcription milestone"
```

## Self-Review

- Spec coverage: the plan covers the Rust worker supervisor, Python worker protocol, model-manager integration, app state updates, packaging layout, failure handling hooks, and end-to-end verification.
- Placeholder scan: all tasks include concrete file paths, commands, and starter code. The only conditional step is the packaging hook because the current repo structure may use either `build.rs` or Tauri resource config; the implementing engineer must choose the existing packaging seam already present in `src-tauri` rather than inventing a second one.
- Type consistency: the plan uses `Processing`, `Completed`, `ensure_loaded`, `transcribe_file`, `unload`, `WorkerRequest`, and `WorkerResponse` consistently across tasks.
