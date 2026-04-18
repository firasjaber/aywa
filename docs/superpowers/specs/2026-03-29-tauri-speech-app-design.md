# OK — Local Speech-to-Text App Design Spec

## Overview

A lightweight Windows tray app built with Tauri v2 + Rust that provides local speech-to-text transcription. The user holds a hotkey, speaks, and the transcribed text is copied to clipboard and pasted into the focused input.

## Architecture

Monolithic Tauri v2 app with clean internal trait boundaries per concern. One Rust binary, no sidecar worker process. Code is structured so that a separate worker process can be extracted later if needed.

- **Frontend:** SvelteKit + TypeScript
- **Backend:** Rust, one module per concern
- **Inference:** ONNX Runtime via the `ort` crate
- **Models:** NVIDIA Parakeet TDT 0.6B (v2 English, v3 multilingual), ONNX-exported, downloaded from Hugging Face

## Project Structure

```
ok/
├── src-tauri/
│   ├── Cargo.toml
│   ├── tauri.conf.json
│   ├── capabilities/         # Tauri v2 permissions
│   ├── icons/
│   └── src/
│       ├── main.rs           # Entry point, app setup
│       ├── lib.rs            # Tauri command registration
│       ├── state.rs          # App state machine
│       ├── hotkey.rs         # Global hotkey listener
│       ├── audio.rs          # Audio capture (WASAPI via cpal)
│       ├── transcribe.rs     # SpeechBackend trait + ONNX impl
│       ├── model_manager.rs  # Download, verify, install models
│       ├── clipboard.rs      # Copy + paste simulation
│       └── overlay.rs        # Overlay window management
├── src/                      # Svelte frontend
│   ├── app.html
│   ├── app.css
│   ├── lib/
│   │   ├── stores/
│   │   │   └── appState.ts   # Reactive state from backend
│   │   └── components/
│   │       ├── Overlay.svelte
│   │       ├── Settings.svelte
│   │       ├── ModelManager.svelte
│   │       └── TrayMenu.svelte
│   └── routes/
│       ├── +layout.svelte
│       └── +page.svelte
├── package.json
├── svelte.config.js
├── vite.config.ts
└── docs/
```

## State Machine

Single Rust enum, serialized and pushed to the frontend via Tauri events.

```rust
enum AppState {
    FirstRun,
    CheckingAssets,
    DownloadingModel { model_id: String, progress: f32 },
    Ready,
    Listening,
    Processing,
    Completed { text: String },
    Error { message: String },
}
```

### Transitions

```
FirstRun → CheckingAssets → Ready
                          → DownloadingModel → Ready
Ready → Listening → Processing → Completed → Ready
Any state → Error → Ready (after recovery)
```

- **FirstRun:** No config exists. Show settings window with model picker.
- **CheckingAssets:** On startup, verify at least one model is installed and intact via SHA-256 checksum.
- **DownloadingModel:** Downloading/extracting a model. Progress emitted to frontend.
- **Ready:** Model loaded into ONNX session, hotkey active, waiting for input.
- **Listening:** Hotkey held, audio being captured.
- **Processing:** Hotkey released, audio sent to inference on background thread.
- **Completed:** Text copied to clipboard, paste attempted. Auto-transitions to Ready after ~1.5s.
- **Error:** Overlay shows error briefly, transitions back to Ready.

## Audio Capture

- **Crate:** `cpal` (WASAPI backend on Windows)
- **Format:** f32 PCM, mono, 16kHz (Parakeet's expected sample rate)
- **Storage:** In-memory `Vec<f32>` buffer — no temp files. Typical utterances (< 30s) are a few MB.
- **Flow:** Hotkey pressed → open default input device, start capture → hotkey released → stop capture, pass buffer to transcription.
- **Resampling:** If the mic's native rate differs from 16kHz, resample with the `rubato` crate.
- **Error:** If default input device unavailable, transition to Error state.

## Transcription Pipeline

### SpeechBackend Trait

```rust
pub trait SpeechBackend: Send + Sync {
    fn load_model(&mut self, model_path: &Path) -> Result<()>;
    fn transcribe(&self, audio: &[f32], sample_rate: u32) -> Result<String>;
    fn unload(&mut self);
}
```

### ONNX Implementation

- Uses the `ort` crate to create an ONNX `Session`.
- Model loaded once when user selects it (or on startup for the last-used model). Stays in memory until switched or app closes.
- Inference runs on a background thread via `tokio::task::spawn_blocking`.

### Execution Provider Priority

1. **CUDA** — if NVIDIA GPU + drivers detected
2. **DirectML** — Windows fallback, works with AMD/Intel/NVIDIA
3. **CPU**

Detection happens at model load time. If a GPU provider fails to initialize, fall back silently to the next option. Log which provider was selected.

### Parakeet Model I/O

- **Input:** Audio tensor (f32, mono, 16kHz) + audio length tensor
- **Output:** Token IDs decoded to text via the model's tokenizer/vocabulary (bundled with the ONNX export)

## Model Manager

### Storage Layout

```
%APPDATA%/ok/
├── config.json
├── models/
│   ├── parakeet-tdt-0.6b-v2/
│   │   ├── model.onnx
│   │   ├── tokenizer.json
│   │   └── manifest.json
│   └── parakeet-tdt-0.6b-v3/
│       └── ...
└── logs/
```

### Download Source

Models downloaded directly from Hugging Face using the HF Hub HTTP API:
- Metadata: `https://huggingface.co/api/models/{repo_id}`
- Files: `https://huggingface.co/{repo_id}/resolve/main/{filename}`
- No API key needed for public models.
- SHA-256 checksums obtained from HF API for integrity verification.

ONNX-exported versions of the Parakeet models will be hosted on a HF repo (existing community exports or user's own repo).

### Download Flow

1. User picks a model in the UI → DownloadingModel state.
2. Download files with progress events emitted to frontend via `reqwest` streaming.
3. Verify SHA-256 checksums after download.
4. If checksum fails, delete and retry once, then Error.
5. On success, transition to Ready and load the model.

### Startup Integrity Check

On app launch (CheckingAssets state), verify installed models' checksums. If corrupted, mark as uninstalled — user can re-download from the model switcher.

## Hotkey

- **Crate:** `global-hotkey`
- **Binding:** `Ctrl+Alt+Space` (fixed, not user-configurable in v1)
- **Behavior:** Hold-to-talk only. Key-down starts recording, key-up stops recording and begins transcription.
- Active even when the app has no focused window.

## Clipboard & Paste

- **Clipboard:** `arboard` crate — always copy transcription result.
- **Paste:** `enigo` crate — simulate `Ctrl+V` via Windows `SendInput` API.
- If paste simulation fails silently, user still has clipboard as fallback.
- Overlay shows "Copied" regardless of paste success.

## Overlay

A separate Tauri window: small, borderless, always-on-top, transparent background, click-through. Positioned bottom-right, just above the system tray area. Hidden when in Ready state — only appears during active use.

### States

| State | Icon | Text | Duration |
|-------|------|------|----------|
| Listening | Pulsing mic | "Listening..." | While held |
| Processing | Spinner | "Transcribing..." | Until done |
| Completed | Checkmark | "Copied" | ~1.5s |
| Error | Red icon | Brief error text | ~3s |

## Tray Icon

- System tray icon, always visible while app is running.
- **Right-click menu:** Model switcher (installed models + download options), Settings, Quit.
- **Left-click:** Opens settings window.

## Settings Window

Regular Tauri window. Minimal for v1:
- Current model + switcher with download buttons
- Active execution provider (GPU/CPU) — display only
- Download progress when installing a model

## Rust Dependencies

```toml
[dependencies]
tauri = { version = "2", features = ["tray-icon"] }
ort = "2"
cpal = "0.15"
global-hotkey = "0.6"
arboard = "3"
enigo = "0.2"
reqwest = { version = "0.12", features = ["stream"] }
sha2 = "0.10"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
tokio = { version = "1", features = ["full"] }
dirs = "5"
tracing = "0.1"
tracing-subscriber = "0.3"
```

## Frontend Dependencies

- SvelteKit + TypeScript
- `@tauri-apps/api` for event listening and command invocation
- No other frontend dependencies for v1.

## First Milestone

Press `Ctrl+Alt+Space` → speak → release → transcribed text appears in Notepad.

## Development Phases

1. **Scaffold:** Tauri v2 + SvelteKit project, tray icon, overlay window
2. **Hotkey + Audio:** Global hotkey capture, audio recording to memory buffer
3. **Model Manager:** HF download, integrity verification, model storage
4. **Transcription:** ONNX Runtime integration, SpeechBackend implementation
5. **Clipboard + Paste:** Copy result, simulate Ctrl+V
6. **Polish:** Error handling, logging, overlay animations, edge cases
