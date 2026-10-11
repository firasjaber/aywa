# OK - Phase 2 Design: Audio Recording

Reference context: `docs/tauri_speech_app_plan.md`

## Goal

Replace the phase-1 fake transcription flow with real push-to-talk microphone recording.

Phase 2 is complete when pressing the global hotkey starts recording from the Windows default microphone, releasing the hotkey stops recording, and the app saves a valid mono PCM WAV file into a temp/runtime location.

## Scope

Phase 2 includes:

- recording from the Windows default input device only
- push-to-talk start and stop using the existing global hotkey
- in-memory audio buffering during capture
- WAV file creation on hotkey release
- logging the saved file path on success
- recording-specific overlay states
- minimal error handling for missing devices and save failures

Phase 2 does not include:

- microphone picker UI
- persistent background recorder service
- worker process integration
- model loading
- transcription
- clipboard copy
- paste automation
- audio device settings

## Architecture

Phase 2 keeps the existing Tauri app structure and replaces the fake post-release flow with a short-lived recording session.

- `hotkey.rs` still owns the push-to-talk interaction
- `audio.rs` owns default-device lookup, sample collection, and WAV writing
- `state.rs` expands to cover recording-specific states
- `lib.rs` coordinates state transitions and audio session lifetime
- the frontend overlay remains a passive rendering surface driven by backend state events

The recorder should open the Windows default microphone only when the user presses the hotkey. It should not keep the microphone open while the app is idle.

## State Machine

Phase 2 replaces the phase-1 fake completion flow with recording-specific states:

- `Ready`
- `Listening`
- `Saving`
- `Error`

Expected transitions:

- `Ready -> Listening`
- `Listening -> Saving`
- `Saving -> Ready`
- `Listening -> Error`
- `Saving -> Error`
- `Error -> Ready`

There is no `Completed` state in this phase. On success, the app should return directly to `Ready` and hide the overlay.

## User Flow

1. App starts in `Ready` with the overlay hidden.
2. User presses and holds the global hotkey.
3. App selects the Windows default microphone and attempts to start a recording session.
4. If recording starts, app transitions to `Listening` and shows the overlay with `Listening...`.
5. User releases the hotkey.
6. App transitions to `Saving` and updates the overlay to `Saving recording...`.
7. The recording session stops, buffered audio is written to a mono PCM WAV file, and the app logs the saved file path.
8. App transitions back to `Ready` immediately and hides the overlay.
9. If any startup or save step fails, app transitions to `Error`, shows a short error message, logs the failure, and then returns to `Ready`.

## Recording Format

- input device: Windows default microphone only
- channels in saved file: mono
- sample encoding: PCM `i16`
- sample rate: use the selected device input rate and write that exact value into the WAV header
- storage: runtime or temp directory controlled by the app
- naming: one new timestamp-based file per recording

For phase 2, the app should downmix multi-channel input to mono before saving. It should not resample unless required by the chosen capture API.

## Components

Suggested responsibilities:

- `src-tauri/src/audio.rs`
  Defines a recorder session type, starts capture from the default device, buffers samples, converts to mono `i16`, and writes the WAV file on stop.
- `src-tauri/src/state.rs`
  Adds `Saving` and `Error`, updates overlay message mapping, and exposes enough payload for the UI to render state text.
- `src-tauri/src/hotkey.rs`
  Keeps hotkey registration and routes press and release events into recording start and stop logic.
- `src-tauri/src/lib.rs`
  Stores the active recorder handle in managed runtime state and owns the transition flow around start, stop, success, and error recovery.
- `src-tauri/src/main.rs`
  Continues to register managed state, startup setup, and Tauri commands.
- `src/lib/stores/appState.ts`
  Continues subscribing to backend state events.
- `src/lib/components/Overlay.svelte`
  Renders `Listening...`, `Saving recording...`, or an error message.
- `src/lib/components/Settings.svelte`
  Updates the placeholder copy so it describes the new recording behavior.

## Data Flow

Startup flow:

1. Create tray icon.
2. Register `main` and `overlay` windows.
3. Initialize app state as `Ready`.
4. Hide the overlay.
5. Register the global hotkey.

Interaction flow:

1. Hotkey press requests `Ready -> Listening`.
2. If accepted, the backend creates a recorder session using the default input device.
3. The recorder callback appends captured samples into an in-memory buffer.
4. Hotkey release requests `Listening -> Saving`.
5. The backend stops the recorder session and writes the WAV file.
6. On success, the app logs the saved path and transitions `Saving -> Ready`.
7. On failure, the app emits `Error`, logs the failure reason, and then transitions back to `Ready` after a short delay.

## Input Guardrails

To keep phase 2 deterministic:

- ignore press events unless the current state is `Ready`
- ignore release events unless the current state is `Listening`
- if no default microphone is available, fail fast into `Error`
- if recording startup fails, do not enter a half-open session state
- if the capture buffer is empty on stop, treat that as an error and do not create an empty WAV file
- while in `Saving` or `Error`, the hotkey does nothing

These rules keep the recording path single-threaded from the user's point of view and avoid overlapping capture sessions.

## Logging

On successful save, log at minimum:

- full output file path
- sample rate
- sample count or captured duration

On failure, log the failing step:

- default-device lookup
- input config selection
- stream start
- sample callback error
- WAV file creation or finalization

## Error Handling

Error handling remains intentionally small in phase 2.

- If the default microphone cannot be found, show an error overlay message, log the issue, and return to `Ready`.
- If the stream cannot start, show an error overlay message, log the issue, and return to `Ready`.
- If WAV writing fails, show an error overlay message, log the issue, and return to `Ready`.
- If overlay updates fail, log the issue rather than crashing the app.

No retry UI is needed yet. Recovery is simply returning to `Ready` so the user can try again.

## Testing

Manual checks:

1. App starts and shows a tray icon.
2. Tray still opens the settings window.
3. Overlay starts hidden.
4. Holding the hotkey shows `Listening...`.
5. Releasing the hotkey shows `Saving recording...` briefly.
6. The overlay then hides immediately after save.
7. A valid WAV file appears in the runtime or temp directory.
8. The logs include the saved file path.
9. Repeating the flow produces a new file each time.
10. Repeated key presses during `Saving` or `Error` do nothing.
11. If no microphone is available, the app shows an error state and recovers to `Ready` without crashing.

Automated checks should focus on logic that can be tested without real microphone hardware:

- state transition tests
- mono conversion helpers
- output-path generation helpers

Hardware capture should remain a manual verification step in this phase.

## Success Criteria

Phase 2 is complete when:

- the app records from the Windows default microphone
- push-to-talk starts and stops recording reliably
- the overlay reflects the recording state accurately
- the app writes a valid mono PCM WAV file on hotkey release
- the app logs the saved file path on success
- the app recovers cleanly from expected recording errors

## Out of Scope Follow-Up

The next phase can take the saved WAV output from phase 2 and hand it to the worker process for real transcription.
