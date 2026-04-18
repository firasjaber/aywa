# Tauri Speech Phase 1 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build the first real vertical slice of the app: tray, placeholder settings window, overlay window, global hotkey, fake transcription timing, and dummy clipboard copy.

**Architecture:** Keep the existing Tauri + Svelte scaffold and add one small Rust-owned state machine that emits state changes to the frontend. The backend owns hotkey handling, timers, clipboard copy, and overlay visibility; the frontend only renders the current overlay text and a simple settings placeholder.

**Tech Stack:** Tauri v2, Rust, SvelteKit, `global-hotkey`, `arboard`, `serde`, Tauri events

---

## Current Workspace Notes

- The scaffold already exists under `src-tauri/` and `src/`.
- `src-tauri/src/overlay.rs` already positions and hides the overlay window.
- `src-tauri/src/state.rs`, `src-tauri/src/hotkey.rs`, and `src-tauri/src/clipboard.rs` are empty placeholders.
- The workspace is not currently a git repository, so skip commit commands until git is initialized.

## File Map

### Rust backend

- Modify: `src-tauri/Cargo.toml`
  Add phase-1 crates only: `global-hotkey` and `arboard`.
- Modify: `src-tauri/src/main.rs`
  Register managed app state, phase-1 commands, tray wiring, and startup setup.
- Modify: `src-tauri/src/lib.rs`
  Export phase-1 modules, event names, window labels, and the fake-flow orchestration helpers.
- Modify: `src-tauri/src/state.rs`
  Define `AppState`, transition validation, event payload, and unit tests.
- Modify: `src-tauri/src/hotkey.rs`
  Register the global hotkey and forward key press/release events into the fake flow.
- Modify: `src-tauri/src/clipboard.rs`
  Copy a fixed dummy string into the system clipboard.
- Modify: `src-tauri/src/overlay.rs`
  Keep window positioning, but add one helper that toggles visibility based on state.

### Frontend

- Create: `src/lib/stores/appState.ts`
  Subscribe to backend state-change events and expose a readable current state.
- Modify: `src/lib/components/Overlay.svelte`
  Render `Listening...`, `Transcribing...`, or `Copied to clipboard` from the store.
- Modify: `src/lib/components/Settings.svelte`
  Remove the manual overlay-preview button and replace it with placeholder phase-1 copy.

### Verification

- Use: `cargo test` from `src-tauri/`
  Run Rust unit tests for state transitions.
- Use: `npm run check` from repo root
  Validate the Svelte frontend.
- Use: `npm run tauri dev` from repo root
  Manually verify tray, settings, overlay timing, hotkey flow, and clipboard copy.

## Shared Constants

Use these names consistently across tasks.

```rust
pub const APP_STATE_EVENT: &str = "app-state-changed";
pub const DUMMY_TRANSCRIPT: &str = "This is dummy text from OK phase one.";
```

```rust
#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum AppState {
    Ready,
    Listening,
    Processing,
    Completed,
}
```

```ts
export type AppState = 'ready' | 'listening' | 'processing' | 'completed';
```

### Task 1: Add Phase-1 Dependencies And State Core

**Files:**
- Modify: `src-tauri/Cargo.toml`
- Modify: `src-tauri/src/state.rs`
- Modify: `src-tauri/src/lib.rs`
- Test: `src-tauri/src/state.rs`

- [ ] **Step 1: Add the minimal Rust dependencies**

Update `src-tauri/Cargo.toml` dependencies to include the two phase-1 crates only:

```toml
[dependencies]
serde_json = "1.0"
serde = { version = "1.0", features = ["derive"] }
tauri = { version = "2", features = ["tray-icon"] }
global-hotkey = "0.6"
arboard = "3"
```

- [ ] **Step 2: Write the state transition tests first**

Create `src-tauri/src/state.rs` with tests that lock the allowed transitions before implementing helpers:

```rust
#[cfg(test)]
mod tests {
    use super::AppState;

    #[test]
    fn ready_can_start_listening() {
        assert!(AppState::Ready.can_transition_to(AppState::Listening));
    }

    #[test]
    fn listening_can_start_processing() {
        assert!(AppState::Listening.can_transition_to(AppState::Processing));
    }

    #[test]
    fn processing_can_complete() {
        assert!(AppState::Processing.can_transition_to(AppState::Completed));
    }

    #[test]
    fn completed_can_return_to_ready() {
        assert!(AppState::Completed.can_transition_to(AppState::Ready));
    }

    #[test]
    fn invalid_transitions_are_rejected() {
        assert!(!AppState::Ready.can_transition_to(AppState::Processing));
        assert!(!AppState::Listening.can_transition_to(AppState::Completed));
        assert!(!AppState::Processing.can_transition_to(AppState::Listening));
        assert!(!AppState::Completed.can_transition_to(AppState::Processing));
    }
}
```

- [ ] **Step 3: Run the tests to verify they fail**

Run: `cargo test state::tests -- --nocapture`

Expected: FAIL because `AppState` and `can_transition_to` do not exist yet.

- [ ] **Step 4: Implement the minimal state module**

Replace `src-tauri/src/state.rs` with:

```rust
use std::sync::Mutex;

use tauri::{AppHandle, Emitter, Manager, Runtime, State};

use crate::APP_STATE_EVENT;

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum AppState {
    Ready,
    Listening,
    Processing,
    Completed,
}

impl AppState {
    pub fn can_transition_to(self, next: AppState) -> bool {
        matches!(
            (self, next),
            (Self::Ready, Self::Listening)
                | (Self::Listening, Self::Processing)
                | (Self::Processing, Self::Completed)
                | (Self::Completed, Self::Ready)
        )
    }

    pub fn overlay_message(self) -> Option<&'static str> {
        match self {
            Self::Ready => None,
            Self::Listening => Some("Listening..."),
            Self::Processing => Some("Transcribing..."),
            Self::Completed => Some("Copied to clipboard"),
        }
    }
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppStatePayload {
    pub state: AppState,
    pub message: Option<&'static str>,
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

    pub fn transition<R: Runtime>(&self, app: &AppHandle<R>, next: AppState) -> Result<bool, String> {
        let mut current = self.current.lock().map_err(|_| "state mutex poisoned".to_string())?;

        if !current.can_transition_to(next) {
            return Ok(false);
        }

        *current = next;
        let payload = AppStatePayload {
            state: next,
            message: next.overlay_message(),
        };
        app.emit(APP_STATE_EVENT, payload).map_err(|error| error.to_string())?;
        Ok(true)
    }

    pub fn emit_current<R: Runtime>(&self, app: &AppHandle<R>) -> Result<(), String> {
        let state = self.current();
        app.emit(
            APP_STATE_EVENT,
            AppStatePayload {
                state,
                message: state.overlay_message(),
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
        message: current.overlay_message(),
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
    fn listening_can_start_processing() {
        assert!(AppState::Listening.can_transition_to(AppState::Processing));
    }

    #[test]
    fn processing_can_complete() {
        assert!(AppState::Processing.can_transition_to(AppState::Completed));
    }

    #[test]
    fn completed_can_return_to_ready() {
        assert!(AppState::Completed.can_transition_to(AppState::Ready));
    }

    #[test]
    fn invalid_transitions_are_rejected() {
        assert!(!AppState::Ready.can_transition_to(AppState::Processing));
        assert!(!AppState::Listening.can_transition_to(AppState::Completed));
        assert!(!AppState::Processing.can_transition_to(AppState::Listening));
        assert!(!AppState::Completed.can_transition_to(AppState::Processing));
    }
}
```

- [ ] **Step 5: Export the shared constants from `lib.rs`**

Update the top of `src-tauri/src/lib.rs` so the rest of the backend can share the event name, window labels, and dummy text:

```rust
pub mod audio;
pub mod clipboard;
pub mod hotkey;
pub mod model_manager;
pub mod overlay;
pub mod state;
pub mod transcribe;

pub const MAIN_WINDOW_LABEL: &str = "main";
pub const OVERLAY_WINDOW_LABEL: &str = "overlay";
pub const APP_STATE_EVENT: &str = "app-state-changed";
pub const DUMMY_TRANSCRIPT: &str = "This is dummy text from OK phase one.";
```

- [ ] **Step 6: Run the Rust tests again**

Run: `cargo test state::tests -- --nocapture`

Expected: PASS with 5 passing tests.

### Task 2: Wire Startup, Commands, And Overlay Visibility

**Files:**
- Modify: `src-tauri/src/lib.rs`
- Modify: `src-tauri/src/overlay.rs`
- Modify: `src-tauri/src/main.rs`
- Test: `src-tauri/src/state.rs`

- [ ] **Step 1: Add a single helper that syncs the overlay to the current state**

Extend `src-tauri/src/overlay.rs` with a function that hides on `Ready` and shows otherwise:

```rust
use crate::{state::AppState, OVERLAY_WINDOW_LABEL};

pub fn sync_with_state<R: Runtime>(app: &AppHandle<R>, state: AppState) -> tauri::Result<()> {
    match state {
        AppState::Ready => hide_overlay(app),
        AppState::Listening | AppState::Processing | AppState::Completed => show_overlay(app),
    }
}
```

- [ ] **Step 2: Add a backend helper that applies transitions and updates the overlay**

Add this to `src-tauri/src/lib.rs` below `configure_windows`:

```rust
use tauri::{AppHandle, Manager, Runtime, State};

pub fn transition_to<R: Runtime>(app: &AppHandle<R>, next: state::AppState) -> Result<bool, String> {
    let manager: State<'_, state::AppStateManager> = app.state();
    let changed = manager.transition(app, next)?;

    if changed {
        overlay::sync_with_state(app, next).map_err(|error| error.to_string())?;
    }

    Ok(changed)
}
```

- [ ] **Step 3: Hide the overlay during setup and emit the initial state from `main.rs`**

Keep `configure_windows` in `src-tauri/src/lib.rs` focused on window setup and initial overlay visibility:

```rust
pub fn configure_windows<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<()> {
    if let Some(window) = app.get_webview_window(MAIN_WINDOW_LABEL) {
        window.hide()?;
    }

    overlay::configure(app)?;
    overlay::sync_with_state(app, state::AppState::Ready)?;

    Ok(())
}
```

- [ ] **Step 4: Register managed state and the new command in `main.rs`**

Update the builder in `src-tauri/src/main.rs` to manage the app state and expose the current-state command:

```rust
#[tauri::command]
fn get_current_state(state: tauri::State<'_, ok_lib::state::AppStateManager>) -> ok_lib::state::AppStatePayload {
    ok_lib::state::get_current_state(state)
}

fn main() {
    tauri::Builder::default()
        .manage(ok_lib::state::AppStateManager::default())
        .setup(|app| {
            ok_lib::configure_windows(&app.handle())?;
            build_tray(&app.handle())?;

            let state: tauri::State<'_, ok_lib::state::AppStateManager> = app.state();
            let _ = state.emit_current(&app.handle());

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_current_state,
            show_settings,
            show_overlay,
            hide_overlay,
            toggle_overlay
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

- [ ] **Step 5: Verify the backend still compiles**

Run: `cargo test state::tests -- --nocapture`

Expected: PASS. If there is a compile error around state access in `lib.rs`, fix it now before adding hotkey behavior.

### Task 3: Add Clipboard Copy And Hotkey-Driven Fake Flow

**Files:**
- Modify: `src-tauri/src/clipboard.rs`
- Modify: `src-tauri/src/hotkey.rs`
- Modify: `src-tauri/src/lib.rs`
- Modify: `src-tauri/src/main.rs`

- [ ] **Step 1: Add the clipboard helper**

Replace `src-tauri/src/clipboard.rs` with:

```rust
use arboard::Clipboard;

pub fn copy_text(text: &str) -> Result<(), String> {
    let mut clipboard = Clipboard::new().map_err(|error| error.to_string())?;
    clipboard.set_text(text.to_string()).map_err(|error| error.to_string())
}
```

- [ ] **Step 2: Add the fake flow orchestrator**

Add this helper to `src-tauri/src/lib.rs`:

```rust
pub fn run_fake_transcription_flow<R: Runtime>(app: AppHandle<R>) {
    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_secs(2));

        let _ = crate::clipboard::copy_text(DUMMY_TRANSCRIPT);
        let _ = transition_to(&app, state::AppState::Completed);

        std::thread::sleep(std::time::Duration::from_secs(2));
        let _ = transition_to(&app, state::AppState::Ready);
    });
}
```

- [ ] **Step 3: Implement the hotkey module**

Replace `src-tauri/src/hotkey.rs` with:

```rust
use global_hotkey::{
    hotkey::{Code, HotKey, Modifiers},
    GlobalHotKeyEvent, GlobalHotKeyManager,
};
use tauri::{AppHandle, Runtime};

use crate::{run_fake_transcription_flow, state::AppState, transition_to};

pub struct HotkeyHandle {
    _manager: GlobalHotKeyManager,
    _hotkey: HotKey,
}

pub fn register<R: Runtime>(app: AppHandle<R>) -> Result<HotkeyHandle, String> {
    let hotkey = HotKey::new(Some(Modifiers::CONTROL | Modifiers::ALT), Code::Space);
    let manager = GlobalHotKeyManager::new().map_err(|error| error.to_string())?;
    manager.register(hotkey).map_err(|error| error.to_string())?;

    let app_for_events = app.clone();
    GlobalHotKeyEvent::set_event_handler(Some(move |event| {
        if event.id != hotkey.id() {
            return;
        }

        if event.state == global_hotkey::HotKeyState::Pressed {
            let _ = transition_to(&app_for_events, AppState::Listening);
        }

        if event.state == global_hotkey::HotKeyState::Released {
            if transition_to(&app_for_events, AppState::Processing).unwrap_or(false) {
                run_fake_transcription_flow(app_for_events.clone());
            }
        }
    }));

    Ok(HotkeyHandle {
        _manager: manager,
        _hotkey: hotkey,
    })
}
```

The critical behavior here is that the processing flow only starts when `Listening -> Processing` succeeds. That enforces the spec guardrails without extra branching.

- [ ] **Step 4: Keep the hotkey manager alive for the app lifetime**

Add a small holder type to `src-tauri/src/lib.rs`:

```rust
pub struct RuntimeHandles {
    pub hotkey: std::sync::Mutex<Option<hotkey::HotkeyHandle>>,
}

impl Default for RuntimeHandles {
    fn default() -> Self {
        Self {
            hotkey: std::sync::Mutex::new(None),
        }
    }
}
```

Then update `src-tauri/src/main.rs` so setup registers the hotkey and stores the handle if registration succeeds:

```rust
fn main() {
    tauri::Builder::default()
        .manage(ok_lib::state::AppStateManager::default())
        .manage(ok_lib::RuntimeHandles::default())
        .setup(|app| {
            ok_lib::configure_windows(&app.handle())?;
            build_tray(&app.handle())?;

            match ok_lib::hotkey::register(app.handle().clone()) {
                Ok(handle) => {
                    let runtime_handles: tauri::State<'_, ok_lib::RuntimeHandles> = app.state();
                    *runtime_handles.hotkey.lock().expect("hotkey mutex poisoned") = Some(handle);
                }
                Err(error) => {
                    eprintln!("failed to register global hotkey: {error}");
                }
            }

            let state: tauri::State<'_, ok_lib::state::AppStateManager> = app.state();
            let _ = state.emit_current(&app.handle());

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_current_state,
            show_settings,
            show_overlay,
            hide_overlay,
            toggle_overlay
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

- [ ] **Step 5: Run backend checks**

Run: `cargo test`

Expected: PASS. There may still be runtime-only issues with hotkey registration on Windows, but the state tests must stay green.

### Task 4: Add Frontend State Subscription And Simplify The UI

**Files:**
- Create: `src/lib/stores/appState.ts`
- Modify: `src/lib/components/Overlay.svelte`
- Modify: `src/lib/components/Settings.svelte`

- [ ] **Step 1: Create the frontend app-state store**

Add `src/lib/stores/appState.ts`:

```ts
import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { readable } from 'svelte/store';

export type AppState = 'ready' | 'listening' | 'processing' | 'completed';

export type AppStatePayload = {
    state: AppState;
    message: string | null;
};

const initialState: AppStatePayload = {
    state: 'ready',
    message: null,
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

- [ ] **Step 2: Update the overlay component to render real phase-1 text**

Replace `src/lib/components/Overlay.svelte` with:

```svelte
<script lang="ts">
    import { appState } from '$lib/stores/appState';

    const pulseStates = new Set(['listening', 'processing']);

    let state = $state<'ready' | 'listening' | 'processing' | 'completed'>('ready');
    let message = $state<string | null>(null);

    $effect(() => {
        const value = $appState;
        state = value.state;
        message = value.message;
    });
</script>

<svelte:head>
    <title>OK Overlay</title>
</svelte:head>

<div class="overlay-shell" data-state={state}>
    <div class="pill" aria-live="polite">
        <span class:pulse={pulseStates.has(state)} class="dot" aria-hidden="true"></span>
        <span class="label">{message ?? 'Ready'}</span>
    </div>
</div>
```

Keep the existing styles, but rename `.pulse` to `.dot` and move the animation to the `class:pulse` toggle. The important UI behavior is that the text comes only from backend state.

- [ ] **Step 3: Remove the manual overlay-preview controls from settings**

Replace the script and copy at the top of `src/lib/components/Settings.svelte` with a simple placeholder:

```svelte
<svelte:head>
    <title>OK Settings</title>
</svelte:head>

<section class="settings-shell">
    <div class="settings-card">
        <p class="eyebrow">OK speech</p>
        <h1>Settings</h1>
        <p class="copy">
            Phase 1 only includes the tray, the overlay, the global hotkey, and a fake clipboard flow.
        </p>
        <p class="status">Hold Ctrl+Alt+Space to trigger the listening overlay.</p>
        <p class="hint">More settings will arrive in later phases.</p>
    </div>
</section>
```

Remove the button and the Tauri `invoke` logic entirely. The settings window should stop being a manual test panel.

- [ ] **Step 4: Run frontend checks**

Run: `npm run check`

Expected: PASS. Fix any Svelte 5 syntax issues now before doing a full app run.

### Task 5: End-To-End Manual Verification

**Files:**
- No code changes required unless a verification failure exposes a bug.

- [ ] **Step 1: Start the app in development mode**

Run: `npm run tauri dev`

Expected: the Vite dev server starts, the Tauri app launches, and the tray icon appears.

- [ ] **Step 2: Verify startup behavior**

Check these conditions manually:

```text
1. The tray icon is present.
2. Left-click or the Settings menu item opens the settings window.
3. The overlay starts hidden.
```

- [ ] **Step 3: Verify the hotkey flow**

Check this exact sequence manually:

```text
1. Hold Ctrl+Alt+Space.
2. Overlay appears with "Listening...".
3. Release Ctrl+Alt+Space.
4. Overlay changes to "Transcribing..." for 2 seconds.
5. Overlay changes to "Copied to clipboard" for 2 seconds.
6. Overlay hides again.
```

- [ ] **Step 4: Verify the clipboard result**

Open Notepad and paste after the completed state finishes.

Expected pasted text:

```text
This is dummy text from OK phase one.
```

- [ ] **Step 5: Verify the guardrails**

Manually confirm:

```text
1. Extra key presses during "Transcribing..." do nothing.
2. Extra key presses during "Copied to clipboard" do nothing.
3. Releasing the hotkey without a matching listening state does nothing.
```

- [ ] **Step 6: Fix any failures immediately at the source**

If verification fails, fix the smallest relevant file only:

```text
- overlay visibility bug -> src-tauri/src/overlay.rs or src/lib/components/Overlay.svelte
- transition bug -> src-tauri/src/state.rs or src-tauri/src/lib.rs
- clipboard bug -> src-tauri/src/clipboard.rs
- hotkey bug -> src-tauri/src/hotkey.rs
```

## Spec Coverage Check

- Tray icon and placeholder settings window: covered in Tasks 2 and 5.
- Overlay hidden in `Ready`: covered in Tasks 2 and 5.
- `Ready -> Listening -> Processing -> Completed -> Ready`: covered in Tasks 1, 2, 3, and 5.
- Fake `Transcribing...` delay for 2 seconds: covered in Task 3 and verified in Task 5.
- `Copied to clipboard` visible for 2 seconds: covered in Task 3 and verified in Task 5.
- Dummy clipboard copy with no simulated paste: covered in Task 3 and verified in Task 5.
- Ignore invalid hotkey inputs during processing/completed: covered in Tasks 1 and 3, verified in Task 5.

## Self-Review Notes

- No `TODO`, `TBD`, or deferred placeholder tasks remain.
- Function and type names are consistent across tasks: `AppState`, `AppStateManager`, `transition_to`, `run_fake_transcription_flow`, and `get_current_state`.
- Scope stays inside phase 1 and does not pull in audio capture, model management, worker processes, or paste simulation.
