# Dedicated Settings Tab Design

## Goal

Reduce crowding on Aywa's App tab by moving user preferences into a dedicated Settings tab and restoring the main window's earlier default height.

## Navigation

The settings-window navigation contains four tabs in this order:

1. App
2. Settings
3. History
4. Debug

Settings is always available, including before a transcription model has been downloaded or activated. Each tab button has a matching `aria-controls` target, selected state, and tab panel.

## App Tab

The App tab returns to its operational purpose:

- current model/runtime status;
- model download and activation controls;
- App-level action errors;
- help and project links.

The shortcut recorder and History-saving preference are removed from this tab. The App-status view model no longer exposes `showHotkey`, because preference visibility is no longer conditional on transcription readiness.

## Settings Tab

The Settings tab contains two vertically stacked, visually separate cards:

1. **Dictation shortcut** — the existing shortcut recorder, given a clear card heading and short explanation. It retains Change shortcut, Escape-to-cancel, Reset to default, immediate activation, and transactional failure behavior.
2. **History** — the existing History-saving preference card, unchanged in meaning and persistence behavior.

Both cards reuse their current components and continue loading confirmed values from the shared `state/settings.json`. Each card independently owns its loading, saving, rollback, and inline error state. Moving the components does not change backend settings commands, global-hotkey behavior, History storage, or the shape of `settings.json`.

## History Tab

The History tab remains the transcript browser. Only the preference controlling whether future transcripts are saved moves to Settings; saved entries, reading, copying, deleting, pagination, and clearing remain unchanged.

## Window Size

The main Tauri window's configured default height returns from `560` to `420`, exactly reverting the change that originally made room for the History-saving card. The existing `minHeight: 460` remains unchanged, matching the configuration before that height increase. Width, minimum width, resizability, and overlay dimensions do not change.

Because the minimum height is greater than the configured default, the operating system or Tauri may clamp the visible initial height to `460`. The design intentionally restores the prior configuration rather than changing the pre-existing minimum-size policy.

## Error Handling

- Shortcut load, validation, registration, and persistence errors remain inside the shortcut card.
- History-preference load and persistence errors remain inside the History card.
- A failure in one card does not disable or replace the other card.
- Changing tabs does not write settings or discard a confirmed value.

## Testing

Frontend tests and checks cover:

- `SettingsTab` accepts the new `settings` value;
- App-status results no longer carry obsolete `showHotkey` state;
- existing History-preference and shortcut-capture helpers continue to pass;
- Svelte accessibility and type checks pass with the fourth tab and matching panel IDs;
- the production frontend build succeeds.

Repository verification also runs Rust tests to ensure the presentation-only move does not affect settings persistence or shortcut activation. Browser QA checks the compact App tab, the new Settings tab at the restored window dimensions, both preference cards, and tab navigation.

## Documentation

The User Guide and troubleshooting text direct people to the Settings tab for changing or resetting the shortcut and controlling future History saving. References to the App tab are retained only for operational status and model controls.
