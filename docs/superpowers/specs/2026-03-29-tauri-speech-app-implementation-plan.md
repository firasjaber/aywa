# OK — Implementation Plan

Reference spec: `docs/superpowers/specs/2026-03-29-tauri-speech-app-design.md`

Each phase is self-contained and should be completed fully before moving to the next. Each phase ends with a concrete verification step.

---

## Phase 1: Project Scaffold

**Goal:** A running Tauri v2 + SvelteKit app with a system tray icon and two windows (settings + overlay).

**Steps:**

1. Initialize the SvelteKit project with TypeScript and Vite
2. Install and initialize Tauri v2 (`cargo install create-tauri-app` or `npm create tauri-app`)
3. Configure `tauri.conf.json`:
   - App identifier: `com.ok.speech`
   - Two windows: `main` (settings, hidden on launch) and `overlay` (borderless, transparent, always-on-top, click-through)
   - System tray enabled
4. Set up the tray icon in `main.rs`:
   - Right-click menu: "Settings", separator, "Quit"
   - Left-click opens the settings window
5. Create the overlay window configuration:
   - Borderless, transparent, always-on-top
   - Positioned bottom-right above system tray
   - Hidden by default
6. Create minimal Svelte components:
   - `Settings.svelte` — placeholder "Settings" text
   - `Overlay.svelte` — placeholder pill shape
7. Set up the Rust module structure with empty files:
   - `state.rs`, `hotkey.rs`, `audio.rs`, `transcribe.rs`, `model_manager.rs`, `clipboard.rs`, `overlay.rs`
   - `lib.rs` with Tauri command stubs

**Verification:** Run `cargo tauri dev`. The app starts, tray icon appears, right-click shows menu, left-click opens settings window, overlay window can be toggled visible.

---

## Phase 2: State Machine + Event Bridge

**Goal:** The Rust state machine works and state changes are pushed to the Svelte frontend in real time.

**Steps:**

1. Implement `AppState` enum in `state.rs`:
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
2. Create an `AppStateManager` that:
   - Holds current state behind an `Arc<Mutex<>>`
   - Validates transitions (e.g., can only go to Listening from Ready)
   - Emits a Tauri event (`app-state-changed`) on every transition
3. Register the state manager as Tauri managed state
4. Create a Tauri command `get_current_state` so the frontend can poll on startup
5. Create `appState.ts` store in the frontend:
   - Subscribe to `app-state-changed` events via `@tauri-apps/api`
   - Export a reactive Svelte store
6. Wire the overlay component to show/hide based on state:
   - Hidden when `Ready`, `FirstRun`, `CheckingAssets`
   - Visible for `Listening`, `Processing`, `Completed`, `Error`
7. Add a temporary Tauri command `debug_set_state` to manually trigger state changes for testing

**Verification:** Use `debug_set_state` to cycle through states. Overlay appears/disappears correctly. Frontend store updates reactively.

---

## Phase 3: Global Hotkey + Audio Capture

**Goal:** Holding `Ctrl+Alt+Space` records audio from the microphone into an in-memory buffer.

**Steps:**

1. Add `global-hotkey` and `cpal` to `Cargo.toml`
2. Implement `hotkey.rs`:
   - Register `Ctrl+Alt+Space` on app startup
   - On key-down: emit event to start recording, transition state to `Listening`
   - On key-up: emit event to stop recording, transition state to `Processing`
3. Implement `audio.rs`:
   - Open default input device via `cpal`
   - Capture audio as f32 PCM samples into a `Vec<f32>` buffer
   - Target: mono, 16kHz. If device sample rate differs, note it (resampling added in Phase 5)
   - Provide `start_recording()` and `stop_recording() -> Vec<f32>` interface
4. Wire hotkey events to audio capture in `lib.rs`:
   - Key-down → `start_recording()`
   - Key-up → `stop_recording()` → hold buffer for transcription
5. Update state transitions: Ready → Listening → Processing
6. Update overlay UI:
   - Listening state: pulsing mic icon, "Listening..."
   - Processing state: spinner, "Transcribing..." (no actual transcription yet)

**Verification:** Press and hold `Ctrl+Alt+Space`. Overlay shows "Listening...". Release. Overlay shows "Transcribing..." then transitions to Ready. Audio buffer is captured (log its length/duration).

---

## Phase 4: Model Manager + HuggingFace Downloads

**Goal:** Users can browse available models, download them from HuggingFace, and the app verifies integrity.

**Steps:**

1. Add `reqwest` (with `stream` feature), `sha2`, and `dirs` to `Cargo.toml`
2. Define the model registry in `model_manager.rs`:
   - Hardcoded list of available models (id, name, HF repo, files with paths)
   - Parakeet TDT 0.6B v2 (English) and v3 (multilingual)
3. Implement `ModelManager`:
   - `get_available_models()` — returns list of models with install status
   - `get_installed_models()` — scans `%APPDATA%/ok/models/`
   - `download_model(model_id)` — streams files from HF, emits progress via state machine, verifies SHA-256
   - `verify_model(model_id)` — checks file checksums against expected values
   - `delete_model(model_id)` — removes model directory
   - `get_model_path(model_id)` — returns path to installed model directory
4. Implement `config.json` management:
   - Store active model ID, preferred execution provider
   - Read/write from `%APPDATA%/ok/config.json`
5. Create Tauri commands:
   - `list_models` — returns available models with install status
   - `download_model` — starts download, progress via events
   - `delete_model`
   - `get_active_model` / `set_active_model`
6. Build the `ModelManager.svelte` component:
   - List available models with "Download" / "Delete" / "Active" buttons
   - Show download progress bar during download
7. Build the `Settings.svelte` page:
   - Embed ModelManager component
   - Show current execution provider (placeholder for now)
8. Implement startup flow:
   - `CheckingAssets`: verify active model is installed and intact
   - If no model installed: transition to `FirstRun`, show settings with model picker
   - If model OK: transition to `Ready`

**Verification:** App starts, shows FirstRun. User can download a model (or a test file to verify the download pipeline). Progress bar works. Model files land in `%APPDATA%/ok/models/`. Restart → CheckingAssets → Ready.

---

## Phase 5: ONNX Transcription Pipeline

**Goal:** Audio buffers are transcribed to text using the Parakeet ONNX model.

**Steps:**

1. Add `ort` to `Cargo.toml` with appropriate features for CUDA and DirectML
2. Add `rubato` to `Cargo.toml` for resampling
3. Implement resampling in `audio.rs`:
   - If captured sample rate != 16kHz, resample using `rubato`
4. Implement `SpeechBackend` trait in `transcribe.rs`:
   ```rust
   pub trait SpeechBackend: Send + Sync {
       fn load_model(&mut self, model_path: &Path) -> Result<()>;
       fn transcribe(&self, audio: &[f32], sample_rate: u32) -> Result<String>;
       fn unload(&mut self);
   }
   ```
5. Implement `OnnxBackend`:
   - `load_model`: create ONNX session with execution provider priority (CUDA → DirectML → CPU)
   - Log which execution provider was selected
   - `transcribe`: prepare input tensors (audio + length), run inference, decode output tokens using tokenizer
   - `unload`: drop the session
6. Load the tokenizer:
   - Read `tokenizer.json` from the model directory
   - Decode token IDs to text
7. Wire the full pipeline in `lib.rs`:
   - On app startup (after CheckingAssets): load the active model into the backend
   - On hotkey release (Processing state): pass audio buffer to `transcribe()` on a background thread via `spawn_blocking`
   - On transcription complete: transition to Completed with the text
8. Add Tauri command `switch_model`:
   - Unload current model
   - Load new model
   - Update config

**Verification:** Hold hotkey, speak, release. The transcribed text is logged. Overlay shows "Listening..." → "Transcribing..." → "Copied" (text visible in logs). Test with both models if both are downloaded.

---

## Phase 6: Clipboard, Paste & Polish

**Goal:** Transcribed text is copied to clipboard and pasted into the focused app. Overlay is polished. App is end-to-end functional.

**Steps:**

1. Add `arboard` and `enigo` to `Cargo.toml`
2. Implement `clipboard.rs`:
   - `copy_and_paste(text)`:
     1. Copy text to clipboard via `arboard`
     2. Small delay (~50ms) to ensure clipboard is ready
     3. Simulate `Ctrl+V` via `enigo`
   - If paste fails, text is still on clipboard — no error state needed
3. Wire clipboard into the pipeline:
   - After transcription completes (Completed state): call `copy_and_paste(text)`
4. Implement auto-transition timers:
   - Completed → Ready after ~1.5s
   - Error → Ready after ~3s
5. Polish the overlay:
   - Pill shape with rounded corners, semi-transparent background
   - Icons per state (mic, spinner, checkmark, error)
   - Smooth show/hide transitions
6. Polish the tray menu:
   - Add model switcher submenu (list installed models, active indicator)
   - Add "Download Models..." option that opens settings
7. Add `tracing` logging throughout:
   - Log state transitions, model loads, audio capture duration, transcription time, execution provider
   - Write logs to `%APPDATA%/ok/logs/`
8. Error handling sweep:
   - No mic available → Error state with helpful message
   - Download interrupted → clean up partial files, allow retry
   - Model load failure → Error state, suggest re-downloading
   - Transcription failure → Error state with message
9. Remove `debug_set_state` command

**Verification:** Open Notepad. Press `Ctrl+Alt+Space`, speak, release. Text appears in Notepad. Overlay shows the full Listening → Transcribing → Copied flow. Logs are written. Tray menu works. Error states display correctly when mic is unplugged or model is missing.

---

## Phase Summary

| Phase | Focus | Key Crates Added | Deliverable |
|-------|-------|-------------------|-------------|
| 1 | Scaffold | `tauri`, `sveltekit` | Running app with tray + windows |
| 2 | State machine | — | Reactive state bridge Rust ↔ Svelte |
| 3 | Hotkey + Audio | `global-hotkey`, `cpal` | Hold-to-talk audio capture |
| 4 | Model manager | `reqwest`, `sha2`, `dirs` | HF model download + verification |
| 5 | Transcription | `ort`, `rubato` | Working speech-to-text pipeline |
| 6 | Clipboard + Polish | `arboard`, `enigo`, `tracing` | End-to-end functional app |
