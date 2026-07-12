# Implemented Phases Reference

This document summarizes the currently implemented parts of the app from `docs/tauri_speech_app_plan.md`:

- Phase 1: UI + hotkey + fake text shell
- Phase 2: Audio recording
- Phase 3: Worker process
- Phase 4: Model manager
- Phase 5: Real transcription

Note: the original phase-1 fake text behavior has since been replaced by the real recording flow from phase 2, but the shell architecture from phase 1 is still the foundation of the app.

## Phase 1: UI + Hotkey + Shell Flow

### Summary
The app already has the desktop shell in place: a tray-based Tauri app, a hidden settings window, a separate overlay window, a global push-to-talk hotkey, and a backend-owned app state that drives the UI.

### How It Works
- On startup, `src-tauri/src/main.rs` builds the tray, configures windows, registers the global hotkey, and emits initial backend state to the frontend.
- The tray left-click and tray menu both reopen the settings window.
- The hotkey is `Ctrl+Alt+Space` and is handled fully in Rust.
- The frontend does not own the runtime flow. It listens for backend events and renders the current state.

### Architecture

Backend:
- `src-tauri/src/main.rs`
  - Tauri bootstrap, tray wiring, command registration, startup orchestration.
- `src-tauri/src/lib.rs`
  - Shared orchestration helpers for window configuration, state transitions, overlay control, and runtime handle storage.
- `src-tauri/src/hotkey.rs`
  - Registers the global hotkey and forwards press/release events into the runtime flow.
- `src-tauri/src/state.rs`
  - Implements the app state machine with `Ready`, `Listening`, `Processing`, `Completed`, and `Error`.
  - Emits `app-state-changed` events for the frontend.
- `src-tauri/src/overlay.rs`
  - Positions the overlay window and controls when it is shown or hidden.

Frontend:
- `src/lib/stores/appState.ts`
  - Subscribes to `app-state-changed` and exposes current backend state as a Svelte store.
- `src/lib/components/Overlay.svelte`
  - Renders the overlay pill and status text.
- `src/lib/components/Settings.svelte`
  - Hosts the settings screen.
- `src/routes/+page.svelte`
  - Main settings page.
- `src/routes/overlay/+page.svelte`
  - Dedicated overlay route for the overlay window.

### Phase 1 Status in Current Code
The shell and event-driven architecture from phase 1 are implemented and still active. The original fake text completion path is no longer the runtime behavior, because phase 2 replaced it with real microphone recording.

## Phase 2: Audio Recording

### Summary
Push-to-talk recording is implemented with the system default microphone. Pressing the hotkey starts capture, releasing it stops capture, converts the audio to mono PCM, and saves it as a WAV file in the app runtime directory before handing that file off to the transcription worker.

### How It Works
- Hotkey press calls `start_recording(...)`.
- The app transitions from `Ready` to `Listening`.
- A `RecorderSession` starts capturing from the default input device.
- Hotkey release calls `stop_recording(...)`.
- The app transitions from `Listening` to `Processing`.
- The capture stream stops, buffered samples are collected, downmixed to mono if needed, and written as a `.wav` file.
- The saved file becomes the input to the transcription pipeline.
- If recording fails, the app enters `Error` temporarily and then resets to `Ready`.

### Architecture

Runtime flow:
- `src-tauri/src/hotkey.rs`
  - `Pressed` triggers `start_recording(...)`.
  - `Released` triggers `stop_recording(...)`.
- `src-tauri/src/lib.rs`
  - Owns runtime handles, including the active recorder session.
  - Coordinates state transitions around recording start/stop.
  - Resolves the recordings directory under app-local data.

Audio pipeline:
- `src-tauri/src/audio.rs`
  - Implements `RecorderSession`.
  - Uses `cpal` to capture audio from the default microphone.
  - Supports `i16`, `u16`, and `f32` input sample formats.
  - Buffers captured samples in memory.
  - Downmixes multi-channel audio to mono.
  - Writes 16-bit mono WAV files using `hound`.

State and UX:
- `src-tauri/src/state.rs`
  - Drives the phase-2 interaction with backend-controlled states.
- `src-tauri/src/overlay.rs`
  - Shows the overlay for `Listening`, `Processing`, `Completed`, and `Error`.
- `src/lib/components/Overlay.svelte`
  - Displays the current state message such as `Listening...`, `Transcribing...`, `Pasted`, and error text.

### Storage Layout Used by Phase 2
Recordings are saved under the app local data directory in:

```text
runtime/recordings/
```

The current filename pattern is:

```text
recording-<timestamp>.wav
```

### Current Scope In The App
Phase 2 no longer stops at WAV creation in the shipped app. Recording remains a distinct implemented subsystem, but it now feeds phases 3 and 5 instead of returning directly to `Ready` after file save.

## Phase 3: Worker Process

### Summary
A private Python worker process is implemented and supervised by Rust. It loads one model, accepts transcription requests over JSON lines on stdio, returns results to Rust, and stays hidden behind the Tauri backend.

### How It Works
- Rust resolves the runtime paths for the worker scripts and Python executable.
- On `load_model(...)`, Rust starts the worker with the selected model path.
- The worker loads the model once and emits a `ready` message.
- On transcription, Rust sends a `transcribe` request containing the saved WAV path.
- The worker runs inference and returns either `transcribe_result`, `error`, or `fatal`.
- Rust shuts the worker down on unload, model switch, timeout, or terminal protocol/runtime failure.

### Architecture

Rust supervisor:
- `src-tauri/src/transcribe.rs`
  - Defines worker protocol request and response types.
  - Manages worker spawn, readiness wait, request/response flow, timeout handling, and shutdown.
  - Resolves runtime directories under app-local data.
  - Bootstraps the worker scripts into `runtime/worker`.
- `src-tauri/src/lib.rs`
  - Stores the live transcription worker in `RuntimeHandles`.
  - Uses the worker runtime during model load and recording stop.
- `src-tauri/src/main.rs`
  - Bootstraps the transcription runtime during app setup.

Python worker:
- `src-tauri/python/worker_main.py`
  - Worker entrypoint and stdio protocol loop.
- `src-tauri/python/worker_protocol.py`
  - JSON line parsing and encoding helpers.
- `src-tauri/python/transcriber.py`
  - NeMo model loading and inference adapter.

### Storage Layout Used By Phase 3

```text
runtime/worker/
runtime/python/
```

### Current Scope Limit
The worker runtime is private to the backend and currently supports one loaded model and one in-flight request at a time.

## Phase 4: Model Manager

### Summary
A real model management layer is implemented for the supported Parakeet models. The app can select a model, download it from Hugging Face, verify it, install it into app storage, start or stop the worker-backed runtime for that model, and expose its state to the settings UI.

### How It Works
- On startup, the app reconciles persisted model state from disk.
- The frontend fetches the current model snapshot and then stays in sync through `model-manager-changed` events.
- When a model download is requested, Rust resolves the expected `.nemo` asset from Hugging Face metadata.
- The file is downloaded into cache, verified with SHA-256, and then promoted into the installed models directory.
- Registry state is persisted to disk so the selected and installed model state survives restarts.
- A model is marked as loaded only after the worker successfully starts and reports `ready`.

### Architecture

Catalog and supported models:
- `src-tauri/src/model_manager/catalog.rs`
  - Defines the supported models, labels, source repos, and expected file names.

Core types:
- `src-tauri/src/model_manager/types.rs`
  - Defines `ModelId`, `ModelStatus`, `InstalledModel`, `PreparedModel`, and `RuntimePreflight`.

Manifest resolution:
- `src-tauri/src/model_manager/manifest.rs`
  - Queries the Hugging Face tree API.
  - Finds the expected `.nemo` file for the selected model.
  - Produces a resolved asset descriptor with version, download URL, SHA-256, and size.

Download and install:
- `src-tauri/src/model_manager/download.rs`
  - Downloads the model into `cache/downloads`.
  - Verifies the SHA-256 checksum.
  - Promotes the verified file into the installed models directory.

Persistence:
- `src-tauri/src/model_manager/registry.rs`
  - Stores model state in `state/model_registry.json`.
  - Persists selected model, installed models, model errors, and loaded model.

Runtime preflight and validation:
- `src-tauri/src/model_manager/runtime.rs`
  - Checks for NVIDIA runtime support by probing `nvcuda.dll` and `nvidia-smi`.
  - Validates that a model file exists and has the expected `.nemo` extension.
  - Feeds worker-backed runtime readiness rather than acting as inference by itself.

Service layer:
- `src-tauri/src/model_manager/service.rs`
  - Orchestrates the full model manager lifecycle.
  - Produces `ModelManagerSnapshot` for the UI.
  - Handles startup reconcile, model selection, download, worker-backed load, unload, cache clearing, and event emission.

Frontend:
- `src/lib/stores/modelManager.ts`
  - Fetches the initial model snapshot and subscribes to `model-manager-changed`.
  - Exposes the Tauri actions used by the settings UI.
- `src/lib/components/Settings.svelte`
  - Renders the model manager screen with model rows, statuses, errors, preflight info, and action buttons.

### Storage Layout Used by Phase 4
Phase 4 uses the app local data directory with these subpaths:

```text
state/model_registry.json
cache/downloads/
models/<model-id>/<version>/
```

### Current Scope Limit
The model manager is currently oriented around a single selected model and a single live worker-backed loaded model.

## Phase 5: Real Transcription

### Summary
End-to-end local transcription is implemented. Releasing the push-to-talk hotkey saves a WAV file, sends it to the private Python worker, copies the returned transcript to the clipboard, and attempts to paste it into the focused application.

### How It Works
- The user holds `Ctrl+Alt+Space` to record.
- Releasing the hotkey saves the recording to `runtime/recordings`.
- Rust transitions the app to `Processing` and sends the WAV path to the worker.
- The worker runs NeMo transcription and returns the transcript.
- Rust copies the transcript to the clipboard.
- Rust attempts `Ctrl+V` paste automation.
- The overlay shows `Pasted`, `Copied to clipboard`, or an error before resetting to `Ready`.

### Architecture

Transcription pipeline:
- `src-tauri/src/lib.rs`
  - Replaces the old save-and-finish flow with a full transcription pipeline.
  - Coordinates `Processing`, `Completed`, and `Error` transitions.
- `src-tauri/src/transcribe.rs`
  - Sends transcription requests and waits for correlated responses.
- `src-tauri/src/clipboard.rs`
  - Copies transcript text and simulates `Ctrl+V` paste.
- `src-tauri/src/state.rs`
  - Defines the user-facing transcription states and default messages.

Frontend:
- `src/lib/stores/appState.ts`
  - Tracks `ready | listening | processing | completed | error`.
- `src/lib/components/Overlay.svelte`
  - Shows pulsing activity while listening/transcribing and success/error feedback afterwards.
- `src/lib/components/Settings.svelte`
  - Shows runtime readiness along with model state.

### Storage Layout Used By Phase 5

```text
runtime/recordings/
```

### Current Scope Limit
The current flow supports one transcript at a time and relies on the system default microphone and focused target app for paste behavior.

## Current Overall Architecture

The implemented architecture now looks like this:

1. Tauri shell
   - Tray app
   - Hidden settings window
   - Overlay window
   - Tauri commands and events

2. Rust core
   - Global hotkey registration
   - App state machine
   - Recording lifecycle and WAV output
   - Worker supervision and protocol handling
   - Model manager service and persistence
   - Clipboard copy and paste automation

3. Python worker
   - Private NeMo-backed transcription subprocess
   - JSON-over-stdio request/response loop

4. Svelte frontend
   - Thin UI driven by backend events and snapshots
   - `appState` store for overlay/runtime state
   - `modelManager` store for model lifecycle state

5. Current shipped flow
   - Press hotkey to record
   - Release to save WAV and transcribe locally
   - Copy transcript to clipboard
   - Attempt paste into focused app
