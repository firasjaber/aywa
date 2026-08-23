# History-saving preference design

## Goal

Let a person keep Aywa's local dictation and clipboard workflow while choosing not to retain future transcription text in local History.

## Scope

Add one persisted, app-wide boolean preference: `saveHistory`. It defaults to `true` for new and existing installations. The setting controls only whether completed transcriptions are saved to History.

When `saveHistory` is disabled:

- Aywa still records, transcribes, copies, and attempts to paste text exactly as it does today.
- Aywa does not write a new History transcript or metadata record.
- Existing History entries remain available until the person deletes them individually or uses Clear all history.

This feature does not add retention periods, search, cloud synchronization, or changes to existing deletion behavior.

## Preferences storage and backend contract

A small, dedicated preferences module owns the setting. It stores a versioned JSON file at `<app-local-data>/state/settings.json`, separate from model-registry data. This is Aywa's app-wide settings document and is intentionally extensible for future user preferences.

Its initial shape is:

```json
{
  "version": 1,
  "saveHistory": true
}
```

The module provides:

- a default settings value with `version: 1` and `saveHistory: true`;
- loading that treats a missing preferences file as the default;
- atomic persistence of an updated value; and
- a read operation used by the transcription flow.

Tauri exposes commands to retrieve the current preference and set `saveHistory`. The backend validates the command input as a boolean and returns the persisted value. The frontend never decides whether to save a transcript independently of the backend.

## User interface

The App tab includes a compact History section with a checked-by-default checkbox labelled **Save transcriptions to History**.

Supporting text states: **When off, future transcriptions are not saved. Existing History remains until you clear it.**

The UI loads the persisted value when Settings opens. A change saves immediately. While saving, the control is disabled. If the update fails, the UI restores the last confirmed value and displays a concise inline error; it does not claim that the privacy preference changed.

The existing History tab, individual deletion, and Clear all history controls remain unchanged.

## Transcription behavior

After a successful transcription and before the existing History repository save, Aywa reads `saveHistory`:

- `true`: run the existing History save and retain its current non-fatal failure handling.
- `false`: skip the History save entirely.

Clipboard copying and paste simulation run after this decision and are unaffected. A disabled History setting is not an error and should not add a warning or diagnostic failure.

## Error handling

- A missing preferences file uses `saveHistory: true` and is created only when the person changes the setting.
- An unreadable or invalid preferences file falls back safely to the default and logs the problem; a later successful settings update replaces it with valid data.
- A failed preference write leaves the in-memory and displayed setting unchanged.
- A failed History write while saving is enabled retains the existing behavior: transcription completes and clipboard/paste proceeds.

## Verification

Rust tests cover defaults, missing/invalid-file recovery, round-trip persistence, and atomic-write behavior. A focused transcription-flow test verifies that enabled saving invokes History storage and disabled saving does not.

Frontend tests cover loading the value, successful toggle persistence, disabled-during-save behavior, reverted state after a failed update, and the explanatory copy. Manual verification confirms the preference survives restart, disabled mode produces no new entry, existing entries remain, and dictation still copies/pastes text.
