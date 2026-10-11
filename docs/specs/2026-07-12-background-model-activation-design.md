# Background Model Activation Design

## Goal

Keep the desktop application responsive while a downloaded transcription model is activated, and make activation progress visible to the user.

## Architecture

The model-manager command will become a quick scheduling operation. It will validate that the runtime is ready and that the requested model is installed, mark that model as `Loading` in transient state, emit the existing `model-manager-changed` snapshot event, and then start the existing worker creation and readiness wait on a background Rust thread.

The background task owns the expensive work: model payload validation, worker shutdown/replacement, Python worker startup, the wait for the worker's ready response, and the post-load CUDA probe. It will update the persistent registry and transient status only after the work succeeds or fails, then emit a fresh snapshot. The frontend remains event-driven and does not need to wait for the backend command to resolve before it can render progress.

## State and User Experience

`Loading` is the authoritative backend status for the requested model. A snapshot containing it causes the App status card to say that the named model is being activated and its row to say `Preparing...`.

While any model is `Loading`, every Activate button is disabled. The current activation cannot be replaced or cancelled through the UI. Download controls retain their existing behavior unless they conflict with the active model state.

On success, the transient loading status is removed, the model becomes `Loaded`, the loaded-model registry value is persisted, the live execution device is recorded, and a snapshot announces readiness. On failure, the transient status is removed, no model is marked loaded, an actionable error is persisted for that model, and a snapshot announces the error. The application stays usable throughout; dictation remains unavailable until a loaded model is confirmed ready.

## Error Handling

Requests made while the runtime is bootstrapping, a model is already activating, or the requested model is not installed return a prompt error without scheduling work. A background activation failure follows the same model-error persistence and diagnostics conventions as the current synchronous path. Failures to emit status events are logged and must not prevent cleanup of transient state.

## Testing

Backend unit tests will cover status transitions and reject a second activation while `Loading` is present. Frontend tests will cover the activating App status and the disabled Activate controls based on the snapshot, rather than a component-local pending promise. The normal frontend check and Rust test suite will verify the integration.
