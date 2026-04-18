# Tauri Speech Phase 2 Recording Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the fake phase-1 transcription flow with real push-to-talk recording from the Windows default microphone and save each capture as a mono PCM WAV file.

**Architecture:** Keep the existing Tauri + Svelte structure and replace the fake post-release timer with a short-lived recorder session stored in managed runtime state. The backend owns default-device lookup, capture, WAV writing, and state transitions; the frontend only renders backend-driven overlay text and updates placeholder settings copy.

**Tech Stack:** Tauri v2, Rust, SvelteKit, `global-hotkey`, `cpal`, `hound`, `serde`, Tauri events

---

## Current Workspace Notes

- `src-tauri/src/audio.rs` is empty and is the right place for the recording implementation.
- `src-tauri/src/hotkey.rs` already owns global hotkey press and release handling.
- `src-tauri/src/lib.rs` currently runs a fake transcription flow after hotkey release.
- `src-tauri/src/state.rs` still uses phase-1 states: `Ready`, `Listening`, `Processing`, and `Completed`.
- The overlay frontend already renders backend-provided `message` text from `app-state-changed` events.

## File Map

### Rust backend

- Modify: `src-tauri/Cargo.toml`
  Add `cpal` and `hound` for recording and WAV writing.
- Modify: `src-tauri/src/state.rs`
  Replace phase-1 post-release states with `Saving` and `Error`, and add tests for the new transitions and messages.
- Modify: `src-tauri/src/lib.rs`
  Add recorder runtime storage and replace the fake transcription flow with start/stop recording orchestration.
- Modify: `src-tauri/src/hotkey.rs`
  Keep the same hotkey but call recording start on press and stop on release.
- Create or replace: `src-tauri/src/audio.rs`
  Implement the recorder session, sample conversion helpers, path generation, and WAV writing.
- Modify: `src-tauri/src/overlay.rs`
  Show the overlay for `Listening`, `Saving`, and `Error`; hide it for `Ready`.

### Frontend

- Modify: `src/lib/components/Overlay.svelte`
  Update pulsing states if needed, but keep rendering backend `message` text.
- Modify: `src/lib/components/Settings.svelte`
  Replace phase-1 copy with recording-focused placeholder text.

### Verification

- Use: `cargo test` from `src-tauri/`
  Run Rust unit tests for state logic and audio helper functions.
- Use: `npm run check` from repo root
  Validate the Svelte frontend.
- Use: `npm run tauri dev` from repo root
  Manually verify the Windows default microphone recording flow.

## Shared Types And Constants

Use these names consistently across tasks.

```rust
pub const APP_STATE_EVENT: &str = "app-state-changed";
pub const ERROR_RESET_DELAY_MS: u64 = 1_500;

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum AppState {
    Ready,
    Listening,
    Saving,
    Error,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppStatePayload {
    pub state: AppState,
    pub message: Option<String>,
}
```

```rust
pub struct RuntimeHandles {
    pub hotkey: std::sync::Mutex<Option<hotkey::HotkeyHandle>>,
    pub recorder: std::sync::Mutex<Option<audio::RecorderSession>>,
}
```

```ts
export type AppState = 'ready' | 'listening' | 'saving' | 'error';

export type AppStatePayload = {
    state: AppState;
    message: string | null;
};
```

### Task 1: Add Recording Dependencies And Update App State

**Files:**
- Modify: `src-tauri/Cargo.toml`
- Modify: `src-tauri/src/state.rs`
- Modify: `src-tauri/src/overlay.rs`
- Test: `src-tauri/src/state.rs`

- [ ] **Step 1: Add the audio dependencies**

Update `src-tauri/Cargo.toml` so the dependency block becomes:

```toml
[dependencies]
serde_json = "1.0"
serde = { version = "1.0", features = ["derive"] }
tauri = { version = "2", features = ["tray-icon"] }
global-hotkey = "0.6"
arboard = "3"
cpal = "0.15"
hound = "3.5"
```

- [ ] **Step 2: Write the failing state transition tests first**

Replace the test module in `src-tauri/src/state.rs` with:

```rust
#[cfg(test)]
mod tests {
    use super::AppState;

    #[test]
    fn ready_can_start_listening() {
        assert!(AppState::Ready.can_transition_to(AppState::Listening));
    }

    #[test]
    fn listening_can_start_saving() {
        assert!(AppState::Listening.can_transition_to(AppState::Saving));
    }

    #[test]
    fn saving_can_return_to_ready() {
        assert!(AppState::Saving.can_transition_to(AppState::Ready));
    }

    #[test]
    fn listening_can_fail() {
        assert!(AppState::Listening.can_transition_to(AppState::Error));
    }

    #[test]
    fn saving_can_fail() {
        assert!(AppState::Saving.can_transition_to(AppState::Error));
    }

    #[test]
    fn error_can_return_to_ready() {
        assert!(AppState::Error.can_transition_to(AppState::Ready));
    }

    #[test]
    fn invalid_transitions_are_rejected() {
        assert!(!AppState::Ready.can_transition_to(AppState::Saving));
        assert!(!AppState::Ready.can_transition_to(AppState::Error));
        assert!(!AppState::Listening.can_transition_to(AppState::Ready));
        assert!(!AppState::Saving.can_transition_to(AppState::Listening));
        assert!(!AppState::Error.can_transition_to(AppState::Listening));
    }

    #[test]
    fn overlay_messages_match_phase_two_states() {
        assert_eq!(AppState::Ready.default_message(None), None);
        assert_eq!(
            AppState::Listening.default_message(None),
            Some("Listening...".to_string())
        );
        assert_eq!(
            AppState::Saving.default_message(None),
            Some("Saving recording...".to_string())
        );
        assert_eq!(
            AppState::Error.default_message(Some("No microphone")),
            Some("No microphone".to_string())
        );
    }
}
```

- [ ] **Step 3: Run the tests to verify they fail**

Run: `cargo test state::tests -- --nocapture`

Expected: FAIL because `Saving`, `Error`, and `default_message` do not exist yet.

- [ ] **Step 4: Implement the new state model**

Replace `src-tauri/src/state.rs` with:

```rust
use std::sync::Mutex;

use tauri::{AppHandle, Emitter, Runtime, State};

use crate::APP_STATE_EVENT;

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum AppState {
    Ready,
    Listening,
    Saving,
    Error,
}

impl AppState {
    pub fn can_transition_to(self, next: AppState) -> bool {
        matches!(
            (self, next),
            (Self::Ready, Self::Listening)
                | (Self::Listening, Self::Saving)
                | (Self::Saving, Self::Ready)
                | (Self::Listening, Self::Error)
                | (Self::Saving, Self::Error)
                | (Self::Error, Self::Ready)
        )
    }

    pub fn default_message(self, error_message: Option<&str>) -> Option<String> {
        match self {
            Self::Ready => None,
            Self::Listening => Some("Listening...".to_string()),
            Self::Saving => Some("Saving recording...".to_string()),
            Self::Error => error_message.map(str::to_string),
        }
    }
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppStatePayload {
    pub state: AppState,
    pub message: Option<String>,
}

pub struct AppStateManager {
    current: Mutex<AppState>,
}

impl Default for AppStateManager {
    fn default() -> Self {
        Self {
            current: Mutex::new(AppState::Ready),
        }
    }
}

impl AppStateManager {
    pub fn current(&self) -> AppState {
        *self.current.lock().expect("state mutex poisoned")
    }

    pub fn transition<R: Runtime>(
        &self,
        app: &AppHandle<R>,
        next: AppState,
        message: Option<String>,
    ) -> Result<bool, String> {
        let mut current = self
            .current
            .lock()
            .map_err(|_| "state mutex poisoned".to_string())?;

        if !current.can_transition_to(next) {
            return Ok(false);
        }

        *current = next;
        app.emit(
            APP_STATE_EVENT,
            AppStatePayload {
                state: next,
                message: message.or_else(|| next.default_message(None)),
            },
        )
        .map_err(|error| error.to_string())?;

        Ok(true)
    }

    pub fn emit_current<R: Runtime>(&self, app: &AppHandle<R>) -> Result<(), String> {
        let state = self.current();
        app.emit(
            APP_STATE_EVENT,
            AppStatePayload {
                state,
                message: state.default_message(None),
            },
        )
        .map_err(|error| error.to_string())
    }
}

#[tauri::command]
pub fn get_current_state(state: State<'_, AppStateManager>) -> AppStatePayload {
    let current = state.current();

    AppStatePayload {
        state: current,
        message: current.default_message(None),
    }
}

#[cfg(test)]
mod tests {
    use super::AppState;

    #[test]
    fn ready_can_start_listening() {
        assert!(AppState::Ready.can_transition_to(AppState::Listening));
    }

    #[test]
    fn listening_can_start_saving() {
        assert!(AppState::Listening.can_transition_to(AppState::Saving));
    }

    #[test]
    fn saving_can_return_to_ready() {
        assert!(AppState::Saving.can_transition_to(AppState::Ready));
    }

    #[test]
    fn listening_can_fail() {
        assert!(AppState::Listening.can_transition_to(AppState::Error));
    }

    #[test]
    fn saving_can_fail() {
        assert!(AppState::Saving.can_transition_to(AppState::Error));
    }

    #[test]
    fn error_can_return_to_ready() {
        assert!(AppState::Error.can_transition_to(AppState::Ready));
    }

    #[test]
    fn invalid_transitions_are_rejected() {
        assert!(!AppState::Ready.can_transition_to(AppState::Saving));
        assert!(!AppState::Ready.can_transition_to(AppState::Error));
        assert!(!AppState::Listening.can_transition_to(AppState::Ready));
        assert!(!AppState::Saving.can_transition_to(AppState::Listening));
        assert!(!AppState::Error.can_transition_to(AppState::Listening));
    }

    #[test]
    fn overlay_messages_match_phase_two_states() {
        assert_eq!(AppState::Ready.default_message(None), None);
        assert_eq!(
            AppState::Listening.default_message(None),
            Some("Listening...".to_string())
        );
        assert_eq!(
            AppState::Saving.default_message(None),
            Some("Saving recording...".to_string())
        );
        assert_eq!(
            AppState::Error.default_message(Some("No microphone")),
            Some("No microphone".to_string())
        );
    }
}
```

- [ ] **Step 5: Update overlay visibility for the new states**

Replace `src-tauri/src/overlay.rs` with:

```rust
use tauri::{AppHandle, Manager, PhysicalPosition, Runtime};

use crate::{state::AppState, OVERLAY_WINDOW_LABEL};

const OVERLAY_MARGIN: i32 = 16;

pub fn configure<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<()> {
    let Some(window) = app.get_webview_window(OVERLAY_WINDOW_LABEL) else {
        return Ok(());
    };

    window.set_ignore_cursor_events(true)?;
    position_overlay(&window)?;
    window.hide()?;

    Ok(())
}

pub fn show_overlay<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<()> {
    let Some(window) = app.get_webview_window(OVERLAY_WINDOW_LABEL) else {
        return Ok(());
    };

    position_overlay(&window)?;
    window.show()?;

    Ok(())
}

pub fn hide_overlay<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<()> {
    if let Some(window) = app.get_webview_window(OVERLAY_WINDOW_LABEL) {
        window.hide()?;
    }

    Ok(())
}

pub fn sync_visibility_for_state<R: Runtime>(app: &AppHandle<R>, state: AppState) -> tauri::Result<()> {
    match state {
        AppState::Ready => hide_overlay(app),
        AppState::Listening | AppState::Saving | AppState::Error => show_overlay(app),
    }
}

pub fn toggle_overlay<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<bool> {
    let Some(window) = app.get_webview_window(OVERLAY_WINDOW_LABEL) else {
        return Ok(false);
    };

    if window.is_visible()? {
        window.hide()?;
        return Ok(false);
    }

    position_overlay(&window)?;
    window.show()?;

    Ok(true)
}

fn position_overlay<R: Runtime>(window: &tauri::WebviewWindow<R>) -> tauri::Result<()> {
    let monitor = window
        .current_monitor()?
        .or(window.primary_monitor()?)
        .or_else(|| window.available_monitors().ok()?.into_iter().next());

    let Some(monitor) = monitor else {
        return Ok(());
    };

    let work_area = monitor.work_area();
    let size = window.outer_size()?;
    let x = work_area.position.x + work_area.size.width as i32 - size.width as i32 - OVERLAY_MARGIN;
    let y =
        work_area.position.y + work_area.size.height as i32 - size.height as i32 - OVERLAY_MARGIN;

    window.set_position(PhysicalPosition::new(x, y))?;

    Ok(())
}
```

- [ ] **Step 6: Run the Rust tests again**

Run: `cargo test state::tests -- --nocapture`

Expected: PASS with all state tests green.

- [ ] **Step 7: Commit the state groundwork**

```bash
git add src-tauri/Cargo.toml src-tauri/src/state.rs src-tauri/src/overlay.rs
git commit -m "feat: add recording phase state flow"
```

### Task 2: Build The Recorder Session In `audio.rs`

**Files:**
- Modify: `src-tauri/src/audio.rs`
- Test: `src-tauri/src/audio.rs`

- [ ] **Step 1: Write failing helper tests for downmixing and file naming**

Replace `src-tauri/src/audio.rs` with this test-first skeleton:

```rust
#[cfg(test)]
mod tests {
    use super::{build_output_path, downmix_to_mono_i16};

    #[test]
    fn stereo_i16_samples_are_downmixed_to_mono() {
        let samples = vec![1000_i16, -1000_i16, 3000_i16, 1000_i16];
        let mono = downmix_to_mono_i16(&samples, 2);
        assert_eq!(mono, vec![0_i16, 2000_i16]);
    }

    #[test]
    fn mono_samples_are_kept_as_is() {
        let samples = vec![12_i16, -8_i16, 33_i16];
        let mono = downmix_to_mono_i16(&samples, 1);
        assert_eq!(mono, samples);
    }

    #[test]
    fn output_path_uses_wav_extension_and_prefix() {
        let path = build_output_path(std::path::Path::new("C:/temp"), 1_713_024_800_123);
        assert_eq!(path.parent().unwrap(), std::path::Path::new("C:/temp"));
        assert_eq!(path.extension().unwrap(), "wav");
        assert!(path.file_name().unwrap().to_string_lossy().starts_with("recording-"));
    }
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test audio::tests -- --nocapture`

Expected: FAIL because `build_output_path` and `downmix_to_mono_i16` do not exist yet.

- [ ] **Step 3: Implement the full recorder session**

Replace `src-tauri/src/audio.rs` with:

```rust
use std::{
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    time::{SystemTime, UNIX_EPOCH},
};

use cpal::{
    traits::{DeviceTrait, HostTrait, StreamTrait},
    SampleFormat, SampleRate, Stream, StreamConfig,
};

#[derive(Debug, Clone)]
pub struct RecordingResult {
    pub path: PathBuf,
    pub sample_rate: u32,
    pub sample_count: usize,
}

pub struct RecorderSession {
    stream: Stream,
    sample_rate: u32,
    channels: u16,
    samples: Arc<Mutex<Vec<i16>>>,
}

impl RecorderSession {
    pub fn start(output_dir: &Path) -> Result<Self, String> {
        let host = cpal::default_host();
        let device = host
            .default_input_device()
            .ok_or_else(|| "No default microphone available".to_string())?;

        let supported_config = device
            .default_input_config()
            .map_err(|error| format!("Failed to read default input config: {error}"))?;

        let sample_rate = supported_config.sample_rate().0;
        let channels = supported_config.channels();
        let stream_config = StreamConfig {
            channels,
            sample_rate: SampleRate(sample_rate),
            buffer_size: cpal::BufferSize::Default,
        };

        let samples = Arc::new(Mutex::new(Vec::new()));
        let samples_for_stream = Arc::clone(&samples);

        let error_callback = |error| {
            eprintln!("recording stream error: {error}");
        };

        let stream = match supported_config.sample_format() {
            SampleFormat::I16 => build_i16_stream(&device, &stream_config, samples_for_stream, error_callback),
            SampleFormat::U16 => build_u16_stream(&device, &stream_config, samples_for_stream, error_callback),
            SampleFormat::F32 => build_f32_stream(&device, &stream_config, samples_for_stream, error_callback),
            other => return Err(format!("Unsupported input sample format: {other:?}")),
        }?;

        stream
            .play()
            .map_err(|error| format!("Failed to start input stream: {error}"))?;

        let _ = output_dir;

        Ok(Self {
            stream,
            sample_rate,
            channels,
            samples,
        })
    }

    pub fn stop(self, output_dir: &Path) -> Result<RecordingResult, String> {
        drop(self.stream);

        let raw_samples = self
            .samples
            .lock()
            .map_err(|_| "recording samples mutex poisoned".to_string())?
            .clone();

        if raw_samples.is_empty() {
            return Err("Recording contained no audio samples".to_string());
        }

        std::fs::create_dir_all(output_dir)
            .map_err(|error| format!("Failed to create recording directory: {error}"))?;

        let mono_samples = downmix_to_mono_i16(&raw_samples, self.channels);
        if mono_samples.is_empty() {
            return Err("Recording contained no mono audio samples".to_string());
        }

        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|error| format!("System clock error: {error}"))?
            .as_millis();

        let path = build_output_path(output_dir, timestamp);
        write_wav_file(&path, self.sample_rate, &mono_samples)?;

        Ok(RecordingResult {
            path,
            sample_rate: self.sample_rate,
            sample_count: mono_samples.len(),
        })
    }
}

fn build_i16_stream(
    device: &cpal::Device,
    config: &StreamConfig,
    samples: Arc<Mutex<Vec<i16>>>,
    error_callback: impl FnMut(cpal::StreamError) + Send + 'static,
) -> Result<Stream, String> {
    device
        .build_input_stream(
            config,
            move |data: &[i16], _| {
                if let Ok(mut buffer) = samples.lock() {
                    buffer.extend_from_slice(data);
                }
            },
            error_callback,
            None,
        )
        .map_err(|error| format!("Failed to build i16 input stream: {error}"))
}

fn build_u16_stream(
    device: &cpal::Device,
    config: &StreamConfig,
    samples: Arc<Mutex<Vec<i16>>>,
    error_callback: impl FnMut(cpal::StreamError) + Send + 'static,
) -> Result<Stream, String> {
    device
        .build_input_stream(
            config,
            move |data: &[u16], _| {
                if let Ok(mut buffer) = samples.lock() {
                    buffer.extend(data.iter().map(|sample| (*sample as i32 - 32_768) as i16));
                }
            },
            error_callback,
            None,
        )
        .map_err(|error| format!("Failed to build u16 input stream: {error}"))
}

fn build_f32_stream(
    device: &cpal::Device,
    config: &StreamConfig,
    samples: Arc<Mutex<Vec<i16>>>,
    error_callback: impl FnMut(cpal::StreamError) + Send + 'static,
) -> Result<Stream, String> {
    device
        .build_input_stream(
            config,
            move |data: &[f32], _| {
                if let Ok(mut buffer) = samples.lock() {
                    buffer.extend(data.iter().map(|sample| {
                        let scaled = (*sample * i16::MAX as f32).round();
                        scaled.clamp(i16::MIN as f32, i16::MAX as f32) as i16
                    }));
                }
            },
            error_callback,
            None,
        )
        .map_err(|error| format!("Failed to build f32 input stream: {error}"))
}

fn write_wav_file(path: &Path, sample_rate: u32, samples: &[i16]) -> Result<(), String> {
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };

    let mut writer = hound::WavWriter::create(path, spec)
        .map_err(|error| format!("Failed to create WAV file: {error}"))?;

    for sample in samples {
        writer
            .write_sample(*sample)
            .map_err(|error| format!("Failed to write WAV sample: {error}"))?;
    }

    writer
        .finalize()
        .map_err(|error| format!("Failed to finalize WAV file: {error}"))
}

pub fn downmix_to_mono_i16(samples: &[i16], channels: u16) -> Vec<i16> {
    if channels <= 1 {
        return samples.to_vec();
    }

    samples
        .chunks_exact(channels as usize)
        .map(|frame| {
            let sum: i32 = frame.iter().map(|sample| *sample as i32).sum();
            (sum / frame.len() as i32) as i16
        })
        .collect()
}

pub fn build_output_path(output_dir: &Path, timestamp_ms: u128) -> PathBuf {
    output_dir.join(format!("recording-{timestamp_ms}.wav"))
}

#[cfg(test)]
mod tests {
    use super::{build_output_path, downmix_to_mono_i16};

    #[test]
    fn stereo_i16_samples_are_downmixed_to_mono() {
        let samples = vec![1000_i16, -1000_i16, 3000_i16, 1000_i16];
        let mono = downmix_to_mono_i16(&samples, 2);
        assert_eq!(mono, vec![0_i16, 2000_i16]);
    }

    #[test]
    fn mono_samples_are_kept_as_is() {
        let samples = vec![12_i16, -8_i16, 33_i16];
        let mono = downmix_to_mono_i16(&samples, 1);
        assert_eq!(mono, samples);
    }

    #[test]
    fn output_path_uses_wav_extension_and_prefix() {
        let path = build_output_path(std::path::Path::new("C:/temp"), 1_713_024_800_123);
        assert_eq!(path.parent().unwrap(), std::path::Path::new("C:/temp"));
        assert_eq!(path.extension().unwrap(), "wav");
        assert!(path.file_name().unwrap().to_string_lossy().starts_with("recording-"));
    }
}
```

- [ ] **Step 4: Run the audio helper tests**

Run: `cargo test audio::tests -- --nocapture`

Expected: PASS with the mono conversion and path generation tests green.

- [ ] **Step 5: Commit the recorder implementation**

```bash
git add src-tauri/src/audio.rs
git commit -m "feat: add default microphone recorder"
```

### Task 3: Wire Recording Into Runtime State And Hotkey Flow

**Files:**
- Modify: `src-tauri/src/lib.rs`
- Modify: `src-tauri/src/hotkey.rs`
- Modify: `src-tauri/src/main.rs`

- [ ] **Step 1: Replace the phase-1 runtime and orchestration code**

Replace `src-tauri/src/lib.rs` with:

```rust
pub mod audio;
pub mod clipboard;
pub mod hotkey;
pub mod model_manager;
pub mod overlay;
pub mod state;
pub mod transcribe;

use std::{path::PathBuf, thread, time::Duration};

use tauri::{AppHandle, Manager, Runtime, State};

use crate::{audio::RecorderSession, state::{AppState, AppStateManager}};

pub const MAIN_WINDOW_LABEL: &str = "main";
pub const OVERLAY_WINDOW_LABEL: &str = "overlay";
pub const APP_STATE_EVENT: &str = "app-state-changed";
pub const ERROR_RESET_DELAY_MS: u64 = 1_500;

pub struct RuntimeHandles {
    pub hotkey: std::sync::Mutex<Option<hotkey::HotkeyHandle>>,
    pub recorder: std::sync::Mutex<Option<RecorderSession>>,
}

impl Default for RuntimeHandles {
    fn default() -> Self {
        Self {
            hotkey: std::sync::Mutex::new(None),
            recorder: std::sync::Mutex::new(None),
        }
    }
}

pub fn configure_windows<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<()> {
    if let Some(window) = app.get_webview_window(MAIN_WINDOW_LABEL) {
        window.hide()?;
    }

    overlay::configure(app)?;
    overlay::sync_visibility_for_state(app, AppState::Ready)
}

pub fn transition_to<R: Runtime>(
    app: &AppHandle<R>,
    next: AppState,
    message: Option<String>,
) -> Result<bool, String> {
    let state = app.state::<AppStateManager>();
    let changed = state.transition(app, next, message)?;

    if changed {
        overlay::sync_visibility_for_state(app, next).map_err(|error| error.to_string())?;
    }

    Ok(changed)
}

pub fn recordings_dir<R: Runtime>(app: &AppHandle<R>) -> Result<PathBuf, String> {
    let base = app
        .path()
        .app_local_data_dir()
        .map_err(|error| error.to_string())?;

    Ok(base.join("runtime").join("recordings"))
}

pub fn start_recording<R: Runtime>(app: &AppHandle<R>) -> Result<(), String> {
    if !transition_to(app, AppState::Listening, None)? {
        return Ok(());
    }

    let output_dir = recordings_dir(app)?;
    match RecorderSession::start(&output_dir) {
        Ok(session) => {
            let handles: State<'_, RuntimeHandles> = app.state();
            *handles.recorder.lock().map_err(|_| "recorder mutex poisoned".to_string())? = Some(session);
            Ok(())
        }
        Err(error) => {
            transition_to(app, AppState::Error, Some(error.clone()))?;
            schedule_ready_reset(app.clone());
            Err(error)
        }
    }
}

pub fn stop_recording<R: Runtime>(app: &AppHandle<R>) -> Result<(), String> {
    if !transition_to(app, AppState::Saving, None)? {
        return Ok(());
    }

    let handles: State<'_, RuntimeHandles> = app.state();
    let session = handles
        .recorder
        .lock()
        .map_err(|_| "recorder mutex poisoned".to_string())?
        .take()
        .ok_or_else(|| "No active recorder session".to_string())?;

    let output_dir = recordings_dir(app)?;
    match session.stop(&output_dir) {
        Ok(result) => {
            println!(
                "saved recording to {} (sample_rate={}, sample_count={})",
                result.path.display(),
                result.sample_rate,
                result.sample_count
            );
            let _ = transition_to(app, AppState::Ready, None)?;
            Ok(())
        }
        Err(error) => {
            transition_to(app, AppState::Error, Some(error.clone()))?;
            schedule_ready_reset(app.clone());
            Err(error)
        }
    }
}

fn schedule_ready_reset<R: Runtime>(app: AppHandle<R>) {
    thread::spawn(move || {
        thread::sleep(Duration::from_millis(ERROR_RESET_DELAY_MS));
        let _ = transition_to(&app, AppState::Ready, None);
    });
}

pub fn reveal_settings<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<()> {
    if let Some(window) = app.get_webview_window(MAIN_WINDOW_LABEL) {
        if window.is_minimized()? {
            window.unminimize()?;
        }

        window.show()?;
        window.set_focus()?;
    }

    Ok(())
}

pub fn show_settings(app: AppHandle) -> Result<(), String> {
    reveal_settings(&app).map_err(|error| error.to_string())
}

pub fn show_overlay(app: AppHandle) -> Result<(), String> {
    overlay::show_overlay(&app).map_err(|error| error.to_string())
}

pub fn hide_overlay(app: AppHandle) -> Result<(), String> {
    overlay::hide_overlay(&app).map_err(|error| error.to_string())
}

pub fn toggle_overlay(app: AppHandle) -> Result<bool, String> {
    overlay::toggle_overlay(&app).map_err(|error| error.to_string())
}
```

- [ ] **Step 2: Update the hotkey flow to call start and stop recording**

Replace `src-tauri/src/hotkey.rs` with:

```rust
use global_hotkey::{
    hotkey::{Code, HotKey, Modifiers},
    GlobalHotKeyEvent, GlobalHotKeyManager, HotKeyState,
};
use tauri::{AppHandle, Runtime};

use crate::{start_recording, stop_recording};

pub struct HotkeyHandle {
    _manager: GlobalHotKeyManager,
    _hotkey: HotKey,
}

unsafe impl Send for HotkeyHandle {}
unsafe impl Sync for HotkeyHandle {}

pub fn register<R: Runtime>(app: AppHandle<R>) -> Result<HotkeyHandle, String> {
    let hotkey = HotKey::new(Some(Modifiers::CONTROL | Modifiers::ALT), Code::Space);
    let manager = GlobalHotKeyManager::new().map_err(|error| error.to_string())?;

    manager.register(hotkey).map_err(|error| error.to_string())?;

    let app_for_events = app.clone();

    GlobalHotKeyEvent::set_event_handler(Some(move |event: GlobalHotKeyEvent| {
        if event.id != hotkey.id() {
            return;
        }

        if event.state == HotKeyState::Pressed {
            let _ = start_recording(&app_for_events);
        }

        if event.state == HotKeyState::Released {
            let _ = stop_recording(&app_for_events);
        }
    }));

    Ok(HotkeyHandle {
        _manager: manager,
        _hotkey: hotkey,
    })
}
```

- [ ] **Step 3: Keep `main.rs` aligned with the new runtime state**

Update the imports in `src-tauri/src/main.rs` to stay the same, but verify the `manage(RuntimeHandles::default())` setup remains present exactly as below:

```rust
use ok_lib::{state::AppStateManager, RuntimeHandles};

fn main() {
    tauri::Builder::default()
        .manage(AppStateManager::default())
        .manage(RuntimeHandles::default())
```

No other `main.rs` changes are required in this task.

- [ ] **Step 4: Run the backend test suite**

Run: `cargo test -- --nocapture`

Expected: PASS with both `state` and `audio` tests green.

- [ ] **Step 5: Commit the runtime wiring**

```bash
git add src-tauri/src/lib.rs src-tauri/src/hotkey.rs src-tauri/src/main.rs
git commit -m "feat: wire push-to-talk recording flow"
```

### Task 4: Update Frontend Copy And Overlay Rendering

**Files:**
- Modify: `src/lib/stores/appState.ts`
- Modify: `src/lib/components/Overlay.svelte`
- Modify: `src/lib/components/Settings.svelte`

- [ ] **Step 1: Update the frontend state type**

Replace `src/lib/stores/appState.ts` with:

```ts
import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { readable } from 'svelte/store';

export type AppState = 'ready' | 'listening' | 'saving' | 'error';

export type AppStatePayload = {
    state: AppState;
    message: string | null;
};

const initialState: AppStatePayload = {
    state: 'ready',
    message: null
};

export const appState = readable<AppStatePayload>(initialState, (set) => {
    let unlisten: UnlistenFn | undefined;

    invoke<AppStatePayload>('get_current_state')
        .then(set)
        .catch(() => set(initialState));

    listen<AppStatePayload>('app-state-changed', (event) => {
        set(event.payload);
    })
        .then((cleanup) => {
            unlisten = cleanup;
        })
        .catch(() => set(initialState));

    return () => {
        unlisten?.();
    };
});
```

- [ ] **Step 2: Update the overlay component for the new states**

Replace `src/lib/components/Overlay.svelte` with:

```svelte
<script lang="ts">
    import { appState } from '$lib/stores/appState';

    const pulseStates = new Set(['listening', 'saving']);
</script>

<svelte:head>
    <title>OK Overlay</title>
</svelte:head>

{#if $appState.message}
    <div class="overlay-shell" data-state={$appState.state}>
        <div class="pill" aria-live="polite">
            <span class="dot" class:pulse={pulseStates.has($appState.state)} aria-hidden="true"></span>
            <span class="label">{$appState.message}</span>
        </div>
    </div>
{/if}

<style>
    :global(html),
    :global(body) {
        margin: 0;
        width: 100%;
        height: 100%;
        background: transparent;
        overflow: hidden;
    }

    .overlay-shell {
        width: 100%;
        height: 100%;
        display: flex;
        align-items: center;
        justify-content: flex-end;
        padding: 0.75rem;
        pointer-events: none;
    }

    .pill {
        display: inline-flex;
        align-items: center;
        gap: 0.7rem;
        min-width: 11rem;
        padding: 0.85rem 1.15rem;
        border: 1px solid rgba(255, 255, 255, 0.18);
        border-radius: 999px;
        background: linear-gradient(135deg, rgba(18, 27, 29, 0.9), rgba(33, 48, 52, 0.82));
        box-shadow: 0 16px 32px rgba(4, 10, 11, 0.26);
        backdrop-filter: blur(16px);
        color: #f5f1ea;
    }

    .dot {
        width: 0.7rem;
        height: 0.7rem;
        border-radius: 999px;
        background: #ffb36d;
        box-shadow: 0 0 0 0 rgba(255, 179, 109, 0.45);
    }

    .pulse {
        animation: pulse 1.8s ease-out infinite;
    }

    .label {
        font-size: 0.95rem;
        font-weight: 700;
        letter-spacing: 0.08em;
        text-transform: uppercase;
    }

    @keyframes pulse {
        0% {
            box-shadow: 0 0 0 0 rgba(255, 179, 109, 0.45);
        }

        70% {
            box-shadow: 0 0 0 12px rgba(255, 179, 109, 0);
        }

        100% {
            box-shadow: 0 0 0 0 rgba(255, 179, 109, 0);
        }
    }
</style>
```

- [ ] **Step 3: Update the settings placeholder text**

Replace `src/lib/components/Settings.svelte` with:

```svelte
<svelte:head>
    <title>OK Settings</title>
</svelte:head>

<section class="settings-shell">
    <div class="settings-card">
        <p class="eyebrow">OK speech</p>
        <h1>Settings</h1>
        <p class="copy">
            Phase 2 records audio from the Windows default microphone and saves each
            push-to-talk capture as a WAV file.
        </p>
        <p class="status">Hold Ctrl+Alt+Space to record. Release to save the clip.</p>
        <p class="hint">Microphone selection and transcription arrive in later phases.</p>
    </div>
</section>

<style>
    :global(html),
    :global(body) {
        min-height: 100%;
    }

    :global(body) {
        background:
            radial-gradient(circle at top left, rgba(255, 173, 96, 0.26), transparent 34%),
            linear-gradient(160deg, #f5eee4 0%, #efe6d8 48%, #e8ddd2 100%);
    }

    .settings-shell {
        min-height: 100vh;
        display: grid;
        place-items: center;
        padding: 2rem;
    }

    .settings-card {
        width: min(100%, 36rem);
        padding: 2.25rem;
        border: 1px solid rgba(38, 46, 45, 0.08);
        border-radius: 1.75rem;
        background: rgba(255, 252, 247, 0.88);
        box-shadow: 0 24px 60px rgba(37, 31, 21, 0.14);
        backdrop-filter: blur(18px);
    }

    .eyebrow {
        margin: 0 0 0.75rem;
        font-size: 0.75rem;
        font-weight: 700;
        letter-spacing: 0.18em;
        text-transform: uppercase;
        color: #8a5d21;
    }

    h1 {
        margin: 0;
        font-size: clamp(2.2rem, 5vw, 3.5rem);
        line-height: 0.96;
        letter-spacing: -0.04em;
    }

    .copy,
    .hint,
    .status {
        margin: 0;
        font-size: 1rem;
        line-height: 1.6;
    }

    .copy {
        margin-top: 1rem;
        max-width: 28rem;
        color: #44504c;
    }

    .status {
        margin-top: 1rem;
        padding: 0.85rem 1rem;
        border-radius: 1rem;
    }

    .status {
        background: rgba(46, 73, 71, 0.08);
        color: #234141;
    }

    .hint {
        margin-top: 1.1rem;
        color: #5b6764;
    }

    @media (max-width: 640px) {
        .settings-shell {
            padding: 1rem;
        }

        .settings-card {
            padding: 1.5rem;
            border-radius: 1.4rem;
        }
    }
</style>
```

- [ ] **Step 4: Run the frontend checks**

Run: `npm run check`

Expected: PASS with no Svelte type or compile errors.

- [ ] **Step 5: Commit the frontend updates**

```bash
git add src/lib/stores/appState.ts src/lib/components/Overlay.svelte src/lib/components/Settings.svelte
git commit -m "chore: update UI copy for recording phase"
```

### Task 5: Manual End-To-End Verification

**Files:**
- Use: `src-tauri/src/audio.rs`
- Use: `src-tauri/src/hotkey.rs`
- Use: `src/lib/components/Overlay.svelte`

- [ ] **Step 1: Start the app in dev mode**

Run: `npm run tauri dev`

Expected: The Tauri app launches, tray icon appears, and the settings window stays hidden until opened from the tray.

- [ ] **Step 2: Verify the recording interaction manually**

Manual script:

```text
1. Open Notepad so you have an obvious focused app nearby for testing.
2. Press and hold Ctrl+Alt+Space.
3. Confirm the overlay appears and reads "Listening...".
4. Speak for 2-3 seconds.
5. Release Ctrl+Alt+Space.
6. Confirm the overlay changes to "Saving recording..." briefly.
7. Confirm the overlay hides immediately after the file is saved.
```

Expected: No crash, no stuck overlay, and exactly one recording session per press-release cycle.

- [ ] **Step 3: Verify the output files and logs**

Manual script:

```text
1. Watch the terminal logs.
2. Confirm a line like "saved recording to C:\...\recording-<timestamp>.wav (sample_rate=..., sample_count=...)" appears.
3. Repeat the recording flow once more.
4. Confirm a second WAV path is logged and the filename differs from the first one.
5. Open the recordings folder and confirm both WAV files exist.
```

Expected: Each successful recording creates a distinct `.wav` file and logs its path.

- [ ] **Step 4: Verify the error path**

Manual script:

```text
1. Disable the microphone in Windows, or remove the active input device.
2. Press Ctrl+Alt+Space.
3. Confirm an error overlay message appears.
4. Wait about 1.5 seconds.
5. Confirm the overlay hides and the app returns to idle.
6. Re-enable the microphone and verify recording works again.
```

Expected: The app logs the error, recovers to `Ready`, and can record again without restart.

- [ ] **Step 5: Commit any final fixups required by verification**

```bash
git add src-tauri/src/audio.rs src-tauri/src/lib.rs src-tauri/src/hotkey.rs src-tauri/src/state.rs src/lib/components/Overlay.svelte src/lib/components/Settings.svelte src/lib/stores/appState.ts
git commit -m "fix: polish recording flow verification issues"
```
