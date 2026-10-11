# OK - Phase 1 Design: UI + Hotkey + Fake Text

Reference context: `docs/tauri_speech_app_plan.md`

## Goal

Build the first usable shell of the app: tray icon, placeholder settings window, overlay window, global hotkey, and a fake transcription flow that copies dummy text to the clipboard.

This phase exists to prove the app shell and the core interaction loop without introducing audio capture, model downloads, worker processes, or real speech-to-text.

## Scope

Phase 1 includes:

- system tray icon
- placeholder settings window
- overlay window for transient status text
- global push-to-talk hotkey
- fake processing delay
- dummy clipboard copy
- minimal state machine for the fake interaction flow

Phase 1 does not include:

- audio recording
- worker process
- model manager
- real transcription
- settings UI beyond a placeholder screen
- dedicated error state UI

## Architecture

Phase 1 should be implemented as one Tauri app with two windows and a small Rust-side state controller.

- `main` window: placeholder settings window opened from the tray
- `overlay` window: small always-on-top status UI
- tray icon: opens settings and allows quitting the app
- Rust state machine: owns the phase-1 interaction flow
- frontend UI: renders simple state text only

The overlay should be hidden in `Ready`, shown while listening and processing, remain visible during completion, and hide again after the completed delay finishes.

## State Machine

Phase 1 uses four states only:

- `Ready`
- `Listening`
- `Processing`
- `Completed`

Expected transitions:

- `Ready -> Listening`
- `Listening -> Processing`
- `Processing -> Completed`
- `Completed -> Ready`

No other transitions are needed in this phase.

## User Flow

1. App starts in `Ready` with the overlay hidden.
2. User presses and holds the global hotkey.
3. App transitions to `Listening` and shows the overlay with `Listening...`.
4. User releases the hotkey.
5. App transitions to `Processing` and updates the overlay to `Transcribing...`.
6. App waits 2 seconds to simulate transcription work.
7. App copies a fixed dummy string to the clipboard.
8. App transitions to `Completed` and updates the overlay to `Copied to clipboard`.
9. After 2 seconds, app transitions back to `Ready` and hides the overlay.

## Components

Suggested phase-1 responsibilities:

- `src-tauri/src/state.rs`
  Holds the phase-1 state enum and transition helpers.
- `src-tauri/src/hotkey.rs`
  Registers the global hotkey and routes press and release events into the app flow.
- `src-tauri/src/overlay.rs`
  Shows, hides, and updates the overlay window based on state.
- `src-tauri/src/clipboard.rs`
  Copies a fixed dummy string to the clipboard when fake processing finishes.
- `src-tauri/src/main.rs` or `src-tauri/src/lib.rs`
  Wires tray setup, window creation, state manager, hotkey registration, and the fake processing timer.
- frontend overlay component
  Renders one of three texts: `Listening...`, `Transcribing...`, or `Copied to clipboard`.
- frontend settings window
  Placeholder content only, with no real settings controls.

This split is intentionally small. Each unit has one clear responsibility and maps directly to the phase-1 behavior.

## Data Flow

Startup flow:

1. Create tray icon.
2. Register `main` and `overlay` windows.
3. Initialize app state as `Ready`.
4. Hide the overlay.
5. Register the global hotkey.

Interaction flow:

1. Hotkey press requests `Ready -> Listening`.
2. If accepted, overlay becomes visible and shows `Listening...`.
3. Hotkey release requests `Listening -> Processing`.
4. If accepted, overlay remains visible and shows `Transcribing...`.
5. A background timer waits 2 seconds.
6. When the timer completes, the app copies the dummy text to the clipboard.
7. State advances to `Completed` and the overlay shows `Copied to clipboard`.
8. A second timer waits 2 seconds.
9. State advances to `Ready` and the overlay hides.

## Input Guardrails

To keep phase 1 deterministic:

- ignore release events unless the current state is `Listening`
- ignore press events unless the current state is `Ready`
- while in `Processing` or `Completed`, the hotkey does nothing

These rules avoid accidental re-entry and keep the fake flow easy to reason about.

## Overlay Behavior

The overlay should stay visually simple in this phase.

- Hidden in `Ready`
- Visible in `Listening` with `Listening...`
- Visible in `Processing` with `Transcribing...`
- Visible in `Completed` with `Copied to clipboard`
- Hidden again after `Completed -> Ready`

The overlay does not need advanced styling yet. The goal is clear feedback and correct timing.

## Clipboard Behavior

When fake processing completes, the app copies one fixed dummy string to the clipboard.

The exact dummy text can be defined in implementation, but it should be a stable hardcoded string so manual verification is straightforward.

Phase 1 only requires clipboard copy. It does not attempt simulated paste.

## Error Handling

Error handling should remain minimal in phase 1.

- If hotkey registration fails on startup, log the error and keep the app running so tray and settings still work.
- If clipboard copy fails, log the error and still show the completed overlay message.
- If overlay show or update fails, log the problem rather than crashing the app.

There is no dedicated `Error` state in this phase. That should wait until real backend work exists.

## Testing

Manual checks:

1. App starts and shows a tray icon.
2. Tray can open the placeholder settings window.
3. Overlay starts hidden.
4. Holding the hotkey shows `Listening...`.
5. Releasing the hotkey shows `Transcribing...` for 2 seconds.
6. Then the overlay shows `Copied to clipboard` for 2 seconds.
7. Then the overlay hides.
8. Pasting into Notepad inserts the fixed dummy text from the clipboard.
9. Repeated key presses during `Processing` or `Completed` do nothing.
10. Release without a matching listening state does nothing.

Automated checks are optional in phase 1. If convenient, add small Rust tests for state transitions only. UI automation is out of scope.

## Success Criteria

Phase 1 is complete when:

- the app launches as a Tauri desktop app
- the tray icon works
- the placeholder settings window opens
- the global hotkey drives the fake interaction flow
- the overlay shows the expected messages with the expected timing
- dummy text is copied to the clipboard at the end of processing
- the overlay hides again after completion

## Out of Scope Follow-Up

The next phase can build on this shell by replacing the fake processing path with real audio capture while preserving the same state-driven flow.
