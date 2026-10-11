# OK - Model Manager Design

Reference context:

- `docs/tauri_speech_app_plan.md`
- `docs/superpowers/specs/2026-04-18-tauri-speech-phase-2-recording-design.md`

## Goal

Add a Windows-first model manager that lets the user:

- see the availability of the two supported Parakeet models
- download model assets from the app UI
- know whether a model is ready to use
- explicitly load or unload a model
- start with the English model selected by default

This design covers model metadata, install state, download and verification flow, load and unload orchestration, and readiness checks needed before real transcription is added.

## Supported Models

The app supports exactly two models in this phase:

- `parakeet-v0.2-en`
- `parakeet-v0.3-multilingual`

`parakeet-v0.2-en` is the default selected model on first run.

The user may switch the selected model in settings at any time. Selection does not automatically download or load the model. If the selected model is not installed, the UI should show that it is not ready and require an explicit download action.

## Scope

This phase includes:

- a catalog for the two supported Parakeet models
- app-visible model status for download and load readiness
- persisted selected model state
- persisted installed model metadata
- remote manifest fetch for version, URL, checksum, and archive details
- model download into app-managed storage
- checksum verification before install
- safe install into versioned model directories
- explicit `Download`, `Load`, and `Unload` actions
- startup auto-load of the default model if it is already downloaded and the runtime check passes
- a Windows-first GPU preflight gate for model loading

This phase does not include:

- user-supplied custom models
- automatic model download on selection
- automatic retry loops for failed downloads or loads
- CPU fallback
- worker-process implementation
- actual transcription execution
- automatic installation of NVIDIA drivers or the full CUDA toolkit

## Product Rules

The model manager should enforce these rules:

- only the two known Parakeet models are supported
- multiple models may be installed on disk at the same time
- only one model may be loaded in memory at a time
- the user may unload the currently loaded model without uninstalling it
- the app stays usable if download or load fails
- failures are model-specific and retry is manual
- a model is only ready for transcription when it is loaded and the runtime preflight has passed

## State Model

Model state should separate disk availability from in-memory availability.

Each model derives one visible status from the following set:

- `NotDownloaded`
- `Downloading`
- `Downloaded`
- `Loading`
- `Loaded`
- `Error`

The app should also track these cross-model fields separately:

- `selected_model_id`
- `loaded_model_id: Option<ModelId>`

This separation is important:

- `selected_model_id` is the user preference shown in settings
- `loaded_model_id` is the model currently active for transcription

Examples:

- The multilingual model may be selected but still show `NotDownloaded`.
- Both models may be `Downloaded` on disk, but only one may be `Loaded`.
- `Unload` clears `loaded_model_id` and returns that model to `Downloaded`.

## Storage Layout

The model manager should use the app-local data directory already used by the Tauri backend.

Recommended layout:

```text
AppData/
  models/
    parakeet-v0.2-en/
      <version>/
        ...model files...
    parakeet-v0.3-multilingual/
      <version>/
        ...model files...
  cache/
    downloads/
      ...temporary download files...
  state/
    model_registry.json
```

Requirements:

- installs are versioned per model
- downloads land in cache first
- partial downloads and partial installs are never treated as installed
- the registry file is only updated after a successful verified install

## Model Distribution

The simplest durable distribution strategy is:

- keep a small app-owned manifest document
- host actual model files on Hugging Face
- only download URLs that appear in the manifest

The manifest is the source of truth for:

- model id
- version
- download URL
- checksum
- archive or file layout metadata required for install

This avoids hardcoding file URLs in the app and allows the hosted asset location or version to change without redesigning the manager.

The app should still keep a compiled-in catalog for the two supported models so the settings UI can render known rows even if the manifest fetch is temporarily unavailable.

## Runtime Support Boundary

This phase is Windows-only and supports only users with a compatible NVIDIA GPU environment.

The app should not attempt to automatically install:

- NVIDIA drivers
- the full system CUDA toolkit

Instead, the app owns what it can control:

- shipping or resolving the inference runtime pieces it needs
- checking whether the current machine satisfies the runtime requirements
- allowing or blocking model load based on that preflight result

The model manager should expose a preflight result that can produce a user-facing unsupported state such as `GPU unsupported` or `Runtime unavailable`. The UI should disable `Load` when preflight fails and explain why.

## Components

The backend should keep the model manager small and split into focused units.

### `catalog`

Responsibilities:

- define the two supported model ids
- define labels, default selection, and any fixed metadata the UI needs
- provide stable lookup helpers for known models

### `manifest`

Responsibilities:

- fetch and parse the remote manifest
- resolve model version, URLs, checksums, and layout details
- return an error that keeps downloads unavailable when the manifest cannot be fetched

### `registry`

Responsibilities:

- persist selected model state
- persist installed model metadata
- persist the latest model-specific error details for UI recovery messaging
- rebuild install status from registry plus filesystem inspection on startup

### `model_manager`

Responsibilities:

- orchestrate selection, download, install, load, unload, and startup auto-load
- enforce the single-loaded-model rule
- combine catalog, manifest, registry, disk inspection, and runtime preflight into one status model for the UI
- provide the narrow load boundary that the future worker process will consume

## Frontend Surface

Settings should render one row per supported model.

Each row should show:

- model name
- whether it is the selected model
- current status
- any model-specific error message
- available action buttons

Recommended actions by status:

- `NotDownloaded` -> `Download`
- `Downloading` -> show progress or disabled action
- `Downloaded` -> `Load`
- `Loading` -> show progress or disabled action
- `Loaded` -> `Unload`
- `Error` -> `Retry`

Selection is independent from the action buttons. The user may select a model that is not yet installed.

## Backend Commands And Interfaces

The model manager should expose a small Tauri-facing command surface.

Suggested commands:

- `list_models`
- `select_model`
- `download_model`
- `load_model`
- `unload_model`
- `get_model_status`

The manager should also define a future-facing internal load boundary for the worker layer:

- `prepare_model(model_id) -> PreparedModel`

`PreparedModel` should contain only what the worker needs to load the model later, such as:

- resolved installed path
- model id
- version
- runtime flavor or backend hints

This keeps the worker process dumb with respect to install logic.

## Startup Flow

Startup should follow this order:

1. Run GPU and runtime preflight.
2. Load the local model registry.
3. Inspect installed model directories and reconcile them with the registry.
4. Publish model status to the UI.
5. Fetch or refresh the remote manifest in the background.
6. If the default model is installed and runtime preflight passed, auto-load it.
7. Otherwise leave it in a downloaded-but-unloaded state and show the reason it is not ready.

This keeps startup deterministic and makes the settings screen useful even when network access is unavailable.

## Download And Install Flow

The install flow should be atomic from the app's point of view.

Recommended flow:

1. User clicks `Download` for a model.
2. The manager resolves manifest metadata for that model.
3. The app downloads the payload into `cache/downloads`.
4. The app verifies checksum before install.
5. The app extracts or copies files into `models/<model-id>/<version>/`.
6. The app verifies that the final expected layout exists.
7. The registry is updated to mark that version installed.
8. The visible model status becomes `Downloaded`.

Failure handling requirements:

- checksum mismatch must fail the install
- incomplete cache files must not be promoted to installed state
- incomplete install directories must be deleted or ignored so they cannot be mistaken for valid installs
- the model should remain `NotDownloaded` or move to `Error`, never `Downloaded`, after a failed install

## Load And Unload Flow

Load flow:

1. User clicks `Load`, or startup auto-load triggers for the default model.
2. The manager runs or reuses runtime preflight.
3. If another model is already loaded, unload it first.
4. The manager resolves the installed model path.
5. The manager prepares a `PreparedModel` object and passes it to the inference boundary.
6. On success, `loaded_model_id` is set and the visible model status becomes `Loaded`.

Unload flow:

1. User clicks `Unload`.
2. The manager asks the inference boundary to release the active model.
3. `loaded_model_id` is cleared.
4. The visible model status becomes `Downloaded`.

Load requirements:

- `Load` must be disabled if the model is not installed
- `Load` must be blocked if runtime preflight fails
- loading one model must unload the previously loaded model first
- failed load must not clear a previously healthy loaded model unless unload already succeeded and the user explicitly chose the new load

## Readiness Definition

The rest of the app should consume one clear readiness rule.

A model is:

- `Not downloaded` when no valid installed version exists
- `Downloaded` when files are present and verified, but the model is not active in memory
- `Loaded` when the verified model is active in memory

The app is `ready for transcription` only when:

- exactly one model is `Loaded`
- runtime preflight has passed

That rule should be exposed as a derived readiness check rather than reimplemented across UI and backend code.

## Error Handling

Errors should remain local to the affected model and should not make the rest of the app unusable.

Error categories to handle explicitly:

- manifest fetch failed
- download failed
- checksum mismatch
- extraction or install failed
- unsupported GPU or missing runtime support
- load failed
- unload failed

Behavior:

- show a model-specific error message in settings
- keep unrelated installed models untouched
- allow manual retry
- do not mark a partially downloaded or partially installed model as ready
- do not crash the app because of manifest or model failures

## Testing

Automated Rust tests should focus on logic that can be proven before worker integration:

- catalog identity and default model rules
- manifest parsing
- registry read and write behavior
- status derivation from registry plus filesystem state
- single-loaded-model transition rules
- install path resolution
- checksum verification helpers
- readiness derivation

Manual checks:

1. Fresh install shows both models as `NotDownloaded`.
2. `parakeet-v0.2-en` is selected by default.
3. Selecting `parakeet-v0.3-multilingual` does not auto-download it.
4. Downloading a model ends in `Downloaded` only after verification succeeds.
5. Restarting the app preserves selected model and installed state.
6. If the default model is downloaded and runtime support is valid, startup auto-loads it.
7. Loading one model unloads the previously loaded model.
8. Unloading keeps the model installed.
9. Corrupted files are not treated as installed or ready.
10. Runtime preflight failure blocks load and shows a clear reason.

## Success Criteria

This phase is complete when:

- the settings UI shows both supported Parakeet models
- the English model is selected by default on first run
- the user can explicitly download a model from the app UI
- the app verifies model integrity before treating it as installed
- the user can explicitly load or unload one installed model at a time
- startup auto-loads the default model when it is already installed and the runtime environment is valid
- the app clearly reports whether a model is ready for transcription
- failures remain recoverable through clear status and manual retry

## Out Of Scope Follow-Up

The next phase should connect the model manager's load boundary to the worker process and real transcription pipeline without moving install logic into the worker.
