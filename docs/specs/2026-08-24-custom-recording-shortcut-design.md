# Custom Recording Shortcut Design

## Goal

Allow a person to record a custom global push-to-talk keyboard shortcut from Aywa's Settings window. The shortcut takes effect immediately, persists across launches, and can be reset to the existing `Ctrl+Alt+Space` default.

## Scope

This feature changes only shortcut configuration and presentation. The configured shortcut continues to call the existing recording lifecycle: pressing it starts recording, and releasing it stops recording and starts transcription. Audio capture, transcription, History saving, clipboard copying, and automatic pasting remain unchanged.

Only keyboard shortcuts are supported. A valid shortcut contains at least one modifier (`Ctrl`, `Alt`, `Shift`, or the Windows key) and exactly one non-modifier key. Modifier-only shortcuts and unmodified ordinary keys are rejected.

## Settings Storage

The shortcut is stored in the existing app settings file at `state/settings.json`, alongside the `saveHistory` preference introduced for History saving. Aywa does not create a separate shortcut configuration file.

`AppSettings` gains a `hotkey` string containing a normalized backend representation such as `control+alt+Space`. The default is `control+alt+Space`.

Existing settings files that contain `version` and `saveHistory` but no `hotkey` remain valid. Serde's existing struct default behavior supplies the default shortcut without discarding the saved History preference. The settings schema stays compatible with version 1.

A representative settings file is:

```json
{
  "version": 1,
  "saveHistory": true,
  "hotkey": "control+alt+Space"
}
```

## Backend Architecture

The Rust backend remains authoritative for parsing, normalizing, validating, registering, and persisting shortcuts. Frontend validation exists for fast feedback but is not trusted as the final validation boundary.

The hotkey module owns a live `GlobalHotKeyManager`, the currently registered shortcuts, and a separately tracked active shortcut ID. Its event handler routes press and release events to `start_recording` and `stop_recording` only when an event ID matches the active shortcut ID. Registering a candidate with Windows therefore does not make it an Aywa trigger by itself.

Startup loads `AppSettings` before hotkey registration and attempts to register the saved shortcut. If the saved value cannot be parsed or registered, Aywa logs a sanitized error and attempts to register the default shortcut. When the fallback succeeds, Aywa updates the in-memory setting and atomically persists the default so the UI, active registration, and shared settings file agree. The app continues running even if both registrations fail, matching current startup behavior.

A new Tauri command accepts a shortcut string and applies it transactionally:

1. Parse and normalize the proposed shortcut.
2. Require at least one supported modifier and one non-modifier key.
3. If it is already active, persist its normalized form and return the current settings.
4. Register the proposed shortcut as an inactive candidate while the previous shortcut remains the only active trigger.
5. Save the updated `AppSettings` to the existing `state/settings.json`.
6. Update the in-memory settings and active shortcut ID only after both candidate registration and persistence succeed.
7. Unregister the previous shortcut after the active ID has switched. If this cleanup fails, its events remain ignored because it is no longer the active ID; process exit releases the stale operating-system registration.

If candidate registration or settings persistence fails, Aywa unregisters the candidate and leaves the previous registration, active shortcut ID, in-memory settings, and persisted settings unchanged. The command returns a user-readable error without claiming the new shortcut is active. At no point can a rejected or unsaved candidate start or stop recording.

The reset action calls the same command with the default shortcut; it does not have a separate persistence path.

Shortcut changes are rejected while Aywa is actively listening or processing so that changing the chord cannot strand a recording between its press and release events.

## Settings UI

The existing App-tab hotkey panel becomes an interactive shortcut recorder while retaining its current visual role.

- The panel displays the active shortcut as individual keycaps.
- “Change shortcut” enters capture mode.
- Capture mode displays “Press your shortcut…” and listens to key-down events inside the settings webview.
- Modifier-only key presses keep capture mode open.
- Pressing a non-modifier key completes the candidate chord and immediately sends it to the backend.
- `Escape` cancels capture without changing the shortcut.
- Browser defaults are prevented while capturing so the chord does not activate a webview action.
- Controls are disabled while the backend applies a change.
- “Reset to default” immediately applies `Ctrl+Alt+Space` through the same backend command.

The UI continues displaying the prior active shortcut until the backend confirms success. Invalid chords and registration conflicts appear as concise inline errors. A conflict message explains that another application or Windows may already own the shortcut.

The shortcut display is driven by `get_app_settings`, so reopening Settings always reflects the backend-confirmed persisted value.

## Data Flow

On startup:

1. Load `state/settings.json` into `AppSettingsState`.
2. Read `AppSettings.hotkey`.
3. Parse and register the saved shortcut, with the default as fallback.
4. Keep the resulting hotkey manager alive in `RuntimeHandles`.

On change:

1. The recorder converts a browser keyboard event into a candidate chord.
2. The frontend invokes the shortcut-setting command.
3. Rust validates and transactionally switches the registered global shortcut.
4. Rust writes the complete `AppSettings` object, including both `saveHistory` and `hotkey`, through the existing atomic settings repository.
5. The command returns the confirmed settings, and the UI updates its displayed keycaps.

## Error Handling

- Invalid or unmodified chords are rejected without touching registration or disk state.
- Shortcut conflicts remove the inactive candidate and retain the previous working shortcut and settings.
- Persistence failures remove the inactive candidate and retain the previous registration, active shortcut ID, in-memory settings, and JSON contents.
- Startup parse or registration failures fall back to `Ctrl+Alt+Space`; a successful fallback is saved so Aywa does not display or retry a shortcut that is not active.
- Errors shown in Settings avoid exposing local paths or internal command details.

## Testing

Rust unit tests cover:

- default shortcut configuration;
- loading an older settings file without `hotkey` while preserving `saveHistory`;
- settings round-trip with a custom shortcut;
- shortcut parsing, normalization, and modifier enforcement;
- no-op application of the active shortcut;
- preservation of the previous shortcut when new registration fails;
- rollback behavior when persistence fails.

Frontend unit tests cover:

- normalizing supported modifier and key combinations;
- waiting on modifier-only input;
- rejecting unmodified keys;
- cancelling with `Escape`;
- rendering normalized shortcuts as user-facing key labels.

Verification also runs Rust formatting and tests, Svelte formatting/type checks and tests, and a Windows smoke check for changing, using, restarting with, conflicting, and resetting the shortcut.

## Documentation

The README and User Guide describe the configurable shortcut and its default. Troubleshooting explains how to recover from a shortcut conflict and how to reset the shortcut. Static references that claim Aywa always uses `Ctrl+Alt+Space` are updated where they describe current user-facing behavior.
