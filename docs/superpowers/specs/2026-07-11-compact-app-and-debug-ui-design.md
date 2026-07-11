# Compact App and Debug UI Design

## Purpose

Polish OK’s settings window into a small, calm control surface for local voice typing. Normal users should be able to understand whether dictation is ready, install or switch models, and recover from an installation problem without encountering development terminology. Technical diagnostics remain available for support and issue reports.

## Chosen approach

The window has two tabs:

- **App** (default): daily use, model management, and human-readable state.
- **Debug**: read-only technical diagnostics and support actions.

This was chosen over a single page with collapsible diagnostics and a separate debug window. It keeps the normal workflow compact without making diagnostics hard to find during support.

## App tab

The App tab is one short, vertically ordered page:

1. **Status card** — communicates the current usable state before any model detail.
2. **Models** — shows each supported model and the action appropriate to its state.
3. **Help** — a small setup-guide link. Its destination is intentionally a non-navigating placeholder until the real documentation URLs are supplied.

The app window opens to this tab. It uses the native Windows title bar and an in-app header containing only the OK identity and App/Debug tabs. The default window size is `600 × 640`, it remains resizable, and its minimum size is approximately `520 × 560`. The common ready state should not scroll at the default size.

### Status card

| Condition | Presentation | User action |
| --- | --- | --- |
| Active model, GPU | Calm positive state: `Ready to dictate`; state that NVIDIA GPU acceleration is active. | Hold the displayed fixed shortcut to dictate. |
| Active model, CPU | Yellow informational warning: `Ready — CPU mode`; explain that dictation may be slower and link NVIDIA users to CUDA setup guidance. This is not an error. | Read the CUDA guide if applicable; otherwise dictate normally. |
| No active model | Neutral setup state that directs the user to install a model below. | Download a model. |
| Downloading | Named progress state, for example `Downloading Parakeet Multilingual`. Show simulated progress while the backend download runs. | Wait; do not expose runtime internals. |
| Preparing / activating | Named progress state, for example `Preparing Parakeet Multilingual`, with high-level phase text. | Wait; do not expose runtime internals. |
| Setup/load failure | Plain-language explanation of what failed. Offer setup guidance and a route to view details in Debug. | Use the next valid model action or open Debug. |

The fixed hotkey is shown only as a usage reminder (currently `Ctrl + Alt + Space`). Hotkey configuration is out of scope.

### Models

Every model is a compact row containing its human-readable name, a simple state, and its next action.

- A not-installed model offers **Download**.
- **Download** installs the model without activating it.
- An installed inactive model offers **Activate**. This is the human-facing replacement for the existing developer-oriented Load action.
- The active model is visually marked **Active** and identifies the actual execution device in ordinary language (for example, `Using NVIDIA GPU`).
- An inactive installed model can be made active using a clear action. The normal UI does not expose unload or cache-clearing controls.

While a download is running, the frontend simulates progress from 0% toward 90% and holds there until the backend reports completion. It then advances to 100%. If the download fails, progress resets and the row changes to `Needs attention`. The percentage is a presentation aid and is not added to the backend data contract.

Only high-level model states appear here: Not installed, Downloading, Ready to activate, Preparing, Active, or Needs attention. Raw backend enum names, profile IDs, package versions, and log paths are Debug-only. The frontend keeps a local activating flag while the existing `load_model` command is in flight so the user receives immediate feedback.

## Debug tab

Debug is intentionally read-only. It contains no profile picker, manual runtime selection, test/re-detect/reset actions, cache clearing, or model-management controls.

It presents four scannable diagnostic groups:

1. **System**: operating system, detected GPU, and GPU driver where available.
2. **Acceleration**: CUDA availability/reason, CUDA and PyTorch versions when known, and actual active execution device.
3. **Transcription runtime**: runtime readiness, most recent preparation/check result, and a concise failure reason if present.
4. **Model**: selected/loaded model and its model version or ID.

The only actions are:

- **Open log** — opens the existing diagnostics log file.
- **Copy issue report** — displayed as a disabled future action. Report generation and clipboard behavior are deferred.

`Open log` asks Tauri to reveal the existing diagnostics log in Windows Explorer. If the log path is missing or Explorer cannot be opened, Debug displays a small non-destructive error and remains usable.

## Content and terminology rules

- App copy describes outcomes: `Ready`, `Preparing`, `CPU mode`, and `Needs attention`.
- Debug may use technical terms such as CUDA, PyTorch, driver version, runtime, and model ID.
- CPU fallback must be visible in App as a yellow warning because it affects performance, but it must not imply that the app is broken.
- A successful GPU state should be reassuring but quiet; avoid dev-oriented version strings there.
- If a value is unknown, Debug says `Not detected` or `Not available` rather than displaying a null/empty value.

## Data mapping and interaction boundaries

The existing `ModelManagerSnapshot` supplies most required data. The polished UI maps it as follows:

- `readyForTranscription`, selected/loaded model, and `actualExecutionDevice` determine the App status card.
- Model rows and their existing status events drive the Models section. Frontend-only state supplies simulated download progress and immediate activation feedback.
- Runtime bootstrap messages are converted to high-level App phases; their raw detail remains Debug-only.
- `runtimePreflight`, `workerCudaStatus`, `runtimeProfile.lastTestResult`, currently available model metadata, and `diagnosticsLogPath` populate Debug.
- Basic Windows identity is added through Tauri only if it can be obtained without introducing a broad system-information subsystem. Unknown facts use `Not detected` or `Not available`.
- Existing recovery/profile actions are deliberately omitted from the new frontend surface; this design does not require removing their backend capabilities.

No new system-information dependency is required. On Windows, the backend may expose the platform identity using readily available process/platform information; otherwise the System row displays `Windows` with unavailable details omitted.

## Error handling

App errors must always name the affected item or step and retain the next valid model action. Debug retains the technical reason. If the log path is missing or opening it fails, Debug shows a non-destructive message; Copy issue report remains disabled.

## Verification criteria

- A ready GPU user sees no raw runtime/profile/debug values in App.
- A ready CPU user sees a yellow informational CUDA-help state, not an error.
- Starting a model download installs the model and leaves an explicit **Activate** action.
- A failed activation presents ordinary-language guidance, retains the next valid model action, and preserves technical detail in Debug.
- Debug shows only read-only information plus Open log and Copy issue report.
- Copy issue report is visibly disabled and does not interact with the clipboard.
- Placeholder setup and CUDA links do not navigate.
- The ready App view fits at `600 × 640`, and both tabs remain usable at the minimum window size.

## Designer handoff

Implement the selected Concept 05A visual direction as a compact native-title-bar window. Include App states for GPU-ready, CPU-ready, downloading with simulated progress, activating/preparing, and a plain-language failure. Keep the visual language dark, quiet, high-contrast, and desktop-native, using green for ready, yellow for CPU-mode information, blue/neutral for progress, and red only for an actual failure.
