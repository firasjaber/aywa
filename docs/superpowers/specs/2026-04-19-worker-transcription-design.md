# Worker And Transcription Design

## Scope

This design covers development phases 3 and 5 from `docs/tauri_speech_app_plan.md`:

- phase 3: worker process
- phase 5: real transcription

The goal is to turn the existing push-to-talk recording flow into a real local transcription pipeline while keeping Python fully wrapped behind the Rust/Tauri app.

## Decisions

- Use a private Python worker subprocess managed entirely by Rust.
- Bundle and manage the Python runtime inside the app.
- Keep the worker long-lived and keep one model loaded per worker process.
- Restart the worker on model switch instead of hot-reloading model state in-process.
- Hand off recordings to the worker by WAV file path.
- Use newline-delimited JSON over stdio as the Rust/Python protocol.
- Support one in-flight transcription request at a time for v1.

## Goals

- Keep Python as an internal implementation detail.
- Preserve a single orchestration layer in Rust.
- Reuse the current recording pipeline, which already writes mono WAV files.
- Make worker crashes recoverable without taking down the Tauri app.
- Align loaded-model UI state with a real live inference process.

## Non-Goals

- Supporting multiple concurrent transcription requests.
- Exposing a user-managed standalone Python app or service.
- Hot-reloading models inside the same worker process.
- Final CPU fallback behavior. The design leaves room for it but does not require it for v1.

## Existing Context

The current codebase already has the main seams needed for this work:

- `src-tauri/src/hotkey.rs` starts and stops recording on hotkey press/release.
- `src-tauri/src/audio.rs` records audio and writes mono WAV files to `runtime/recordings`.
- `src-tauri/src/model_manager/service.rs` tracks selected and loaded models.
- `src-tauri/src/transcribe.rs` exists but is empty.
- `src-tauri/src/state.rs` currently exposes `Ready`, `Listening`, `Saving`, and `Error`.

## Architecture

### High-Level Structure

The Tauri main process remains the single orchestrator. It owns the UI-facing state machine, hotkey handling, recording flow, model selection, clipboard integration, and worker lifecycle.

Add a Rust-side `TranscriptionManager` that owns:

- the embedded Python runtime location
- the worker entrypoint path
- the child process handle
- transport over worker stdin/stdout
- current runtime-loaded model identity
- timeout and restart behavior

The Python worker is a private subprocess used only for inference. The frontend never talks to it directly.

### Process Model

Main process responsibilities:

- hotkey handling
- recording control
- model selection and loading intent
- worker startup, shutdown, supervision, and restart
- sending transcription requests
- receiving transcription results
- clipboard copy and paste attempt
- overlay and app-state updates

Worker process responsibilities:

- initialize Python runtime dependencies
- load one selected speech model during startup
- accept transcription requests by audio file path
- run inference
- return transcript text or structured errors

## Worker Lifecycle

### Load Model

`load_model(model_id)` becomes a real runtime operation, not just a registry update.

Flow:

1. Resolve installed model metadata from the model manager registry.
2. Verify the embedded Python runtime exists and matches the expected app-managed version.
3. If another worker is running, terminate it.
4. Spawn a fresh worker configured for the selected model path.
5. Wait for a `ready` message from the worker.
6. Persist `loaded_model_id` only after `ready` is received.

### Unload Model

`unload_model()`:

1. send `shutdown` to the worker if available
2. wait briefly for graceful exit
3. force-kill if needed
4. clear in-memory worker state
5. clear persisted `loaded_model_id`

### Model Switch

Model switching is implemented as teardown plus fresh spawn:

1. terminate old worker
2. spawn a new worker with the new model path
3. persist the new `loaded_model_id` only if startup succeeds

This avoids complexity around Python-side model reload semantics and gives cleaner failure recovery.

## Rust/Python Protocol

Use newline-delimited JSON over stdio.

Rust to Python examples:

```json
{"type":"ping"}
{"type":"transcribe","request_id":"uuid","audio_path":"C:\\...\\recording-123.wav"}
{"type":"shutdown"}
```

Python to Rust examples:

```json
{"type":"ready","model_id":"parakeet-v0.2-en","model_path":"C:\\..."}
{"type":"pong"}
{"type":"transcribe_result","request_id":"uuid","text":"hello world","duration_ms":842}
{"type":"error","request_id":"uuid","message":"failed to decode wav"}
{"type":"fatal","message":"model load failed"}
```

Protocol rules:

- one request in flight at a time for v1
- responses are correlated by `request_id`
- `fatal` means the worker is no longer trusted and must be restarted
- worker stderr is used for logs only, not protocol messages
- malformed or unexpected stdout is treated as worker failure

This protocol stays simple, debuggable, and small because audio is passed by file path rather than inline bytes.

## App State Changes

Replace the current visible state model with a transcription-oriented one:

- `Ready`
- `Listening`
- `Processing`
- `Completed`
- `Error`

Recommended user-facing messages:

- `Listening...`
- `Transcribing...`
- `Copied to clipboard`
- `Pasted`
- specific error text

`Saving` should stop being a product-facing state. Writing the WAV file remains an internal step between recording and transcription.

### State Flow

1. Hotkey press:
   - if the app is `Ready` and a worker-backed model is loaded, start recording
   - if no worker-backed model is loaded, fail fast with a clear error for v1
2. Hotkey release:
   - stop recording and save the WAV file
   - transition to `Processing`
3. Transcription succeeds:
   - copy transcript to clipboard
   - attempt paste
   - show `Completed` briefly with `Pasted` or `Copied to clipboard`
   - return to `Ready`
4. Transcription fails:
   - transition to `Error`
   - if the worker crashed, clear runtime-loaded state
   - after a short delay, return to `Ready`

## Model Manager Integration

The meaning of model manager fields becomes:

- `selected_model_id`: the user's preferred model
- `loaded_model_id`: the model currently backed by a live worker process
- `ready_for_transcription`: a worker is alive, the selected runtime-loaded model is available, and the runtime is healthy

Updated operation semantics:

- `download_model`: unchanged
- `select_model`: unchanged
- `load_model`: install validation plus worker startup
- `unload_model`: worker shutdown plus clearing loaded state

Important persistence rule:

- do not persist `loaded_model_id` until the worker sends `ready`

Startup behavior:

- if the registry indicates a previously loaded model, the app may attempt to restore it
- restoration can be eager during startup or lazy on first use
- v1 should prefer explicit load behavior unless automatic restore proves necessary for UX

## Recording Handoff

The current recording output in `audio.rs` should remain the handoff format.

Flow:

1. recording stops
2. Rust receives a saved WAV path
3. Rust sends a `transcribe` request containing that path
4. worker reads the file directly from disk
5. worker returns transcript text

Reasons for path-based handoff:

- matches the current implementation
- avoids large stdio payloads
- makes requests easy to log and debug
- allows easier retry behavior if needed

## Packaging Layout

Suggested app-data layout:

```text
AppData/
  runtime/
    python/
    worker/
    recordings/
    logs/
  models/
  cache/
  state/
```

Split runtime assets into:

- stable app assets:
  - worker script or package
  - protocol version marker
  - launch wrapper
- managed runtime assets:
  - embedded Python runtime
  - Python packages and environment
  - backend runtime dependencies

Rust owns runtime verification and self-healing:

- verify embedded Python exists
- verify worker package version matches the expected app version or protocol version
- re-extract or rebuild the runtime if corrupted

## Failure Handling

### Worker Startup Failure

If the worker sends `fatal` during startup or exits before `ready`:

- `load_model` fails
- `loaded_model_id` is not persisted
- the model row surfaces an error
- the UI shows an error state or message

### Worker Crash During Idle

If the worker exits unexpectedly while idle:

- clear in-memory worker handle
- mark transcription unavailable
- keep downloaded model metadata intact

### Worker Crash During Transcription

If the worker exits during a request:

- fail the current request
- transition the app to `Error`
- clear runtime-loaded state in memory
- allow a later explicit load or retry to restart the worker

### Timeout

If a request exceeds the configured timeout:

- Rust kills the worker
- Rust clears runtime-loaded state
- the request returns a transcription timeout error

### Protocol Failure

If stdout contains malformed JSON or an unexpected message sequence:

- treat the worker as failed
- terminate it if still running
- surface an error to the caller

## Module Boundaries

### Rust

`src-tauri/src/transcribe.rs` should become the Rust-side worker supervisor and request pipeline module.

It should define:

- worker process handle types
- protocol request and response types
- startup and shutdown logic
- `ensure_loaded`, `transcribe`, and `unload` operations
- timeout and crash handling

`src-tauri/src/model_manager/service.rs` should continue to own the user-facing model lifecycle, but its `load_model` and `unload_model` operations should delegate to the transcription runtime.

`src-tauri/src/state.rs` should be updated to the new visible app states and messages.

### Python

The worker should be small and single-purpose:

- parse JSON lines from stdin
- load model once during startup
- handle `transcribe`
- print JSON lines to stdout
- print logs to stderr
- exit on `shutdown` or unrecoverable fatal errors

## End-To-End Flow

Press-to-paste flow:

1. user presses hotkey
2. Rust verifies the app is ready and begins recording
3. overlay shows `Listening`
4. user releases hotkey
5. Rust stops recording and writes WAV to disk
6. Rust transitions to `Processing`
7. Rust sends `transcribe(audio_path)` to the worker
8. worker runs inference and returns transcript text
9. Rust copies the text to the clipboard
10. Rust attempts paste into the focused input
11. overlay shows `Pasted` or `Copied to clipboard`
12. app returns to `Ready`

## Testing Focus

Key tests for implementation planning:

- worker startup success and failure paths
- model switch teardown and respawn
- transcription request success path
- timeout handling
- worker crash during request
- malformed protocol handling
- state transitions from `Ready` through `Completed` and `Error`
- persistence rule that `loaded_model_id` is only stored after worker `ready`

## Recommended V1 Outcome

The first milestone for phases 3 and 5 should deliver:

- one Rust `TranscriptionManager`
- one private long-lived Python worker subprocess
- one loaded model per worker
- worker restart on model switch
- WAV path handoff
- single in-flight request handling
- clipboard copy and paste on success
- robust error reporting and worker recovery

This is the smallest design that is still production-shaped and consistent with the existing app structure.
