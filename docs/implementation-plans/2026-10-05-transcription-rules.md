# Transcription Rules Implementation Plan

Approved spec: [Transcription Rules](../specs/2026-10-05-transcription-rules.md)

## Goal and existing flow

Let users correct or remove recognized words and phrases through a `Rules` tab. Keep rules local and apply them only to new dictations.

The Python worker returns text to `stop_recording` in `src-tauri/src/lib.rs`. That function currently saves the text to History, copies it to the clipboard, and tries to paste it. Apply rules once at this shared point before those three actions. Keep the existing History preference and paste fallback behavior.

## Approach and constraints

Store the ordered rule list in the existing `AppSettings` document at `state/settings.json`. Add a default empty list so older version 1 files still load. Keep the current atomic save and in-memory rollback behavior. Use one Tauri command to replace the full list after a user edit. Do not put rule contents in diagnostics or support reports.

Use the Rust `regex` crate for Unicode-aware, case-insensitive matching of literal user text. It is already present transitively, but add it as a direct dependency in `src-tauri/Cargo.toml`. Match complete words or phrases, choose the longest match where rules overlap, and scan only the original transcription. Keep replacement spelling and case exactly as entered. Clean spaces and punctuation left by deletions. Do not send rules to the Python worker or change model behavior.

After deletions, remove punctuation that has no remaining text to attach to. Treat a result with no remaining text as a completed dictation with no output. Show `No text after applying Rules` long enough to read. Skip History, clipboard, and paste. Existing History entries stay unchanged.

## Tasks

### 1. Persist and edit rules

Add a serializable rule type with `heardAs` and `useInstead` fields in `src-tauri/src/settings.rs`. Add an empty rule list to `AppSettings` with a serde default. Add a state method that validates and atomically saves a complete replacement list, then changes the in-memory value only after the save succeeds. Reject blank `heardAs` values and duplicate sources that differ only by letter case. Preserve empty `useInstead` values for deletion. Expose this through a `set_rules` command in `src-tauri/src/lib.rs` and `src-tauri/src/main.rs`. Add the matching TypeScript type and call in `src/lib/stores/appSettings.ts`.

Extend the existing settings tests for a version 1 file without rules, rule persistence after reload, invalid input, and failed-save rollback. These tests protect existing preferences and prevent a failed edit from changing active rules.

### 2. Apply rules to dictation output

Add the focused matching code and its tests in `src-tauri/src/transcription_rules.rs`, then call it from `stop_recording` in `src-tauri/src/lib.rs`. Read one confirmed settings snapshot for each completed transcription. Use its rules and `save_history` value for that result. Pass the processed text to History and clipboard, then attempt paste as today. Handle the empty result before any of those actions.

Test a representative correction, a deletion with punctuation, a non-Latin word, case-insensitive matching, a source inside an unrelated word, overlapping phrases, a replacement that also appears as a source rule, and an all-deleted result such as `uhm.`. Keep the tests at the matching and output decision boundary. A short manual check covers the actual Windows clipboard and paste path, which the Rust unit tests cannot observe.

### 3. Add the Rules tab

Add `Rules` to `SETTINGS_TABS` in `src/lib/components/settingsView.ts` and update its existing navigation test. Add a matching tab panel in `src/lib/components/Settings.svelte`. Build the two-column editor in `src/lib/components/Rules.svelte` using `getAppSettings` and `setRules`. Label both inputs, explain that an empty `Use instead` value removes text, and provide add, edit, and delete actions. Load confirmed rules when the tab opens. Save a changed list before showing it as confirmed. Keep the previous confirmed list and show an inline error if saving fails. Provide a retry state if loading fails. Keep the five tab buttons and list usable with a keyboard at the window's minimum size.

Manually check the tab at the configured minimum window size and use the keyboard to add, edit, and remove rules.

### 4. Update help text and verify the flow

Update `docs/USER_GUIDE.md` to explain the Rules tab, the empty replacement field, local storage, and the effect on future dictations. Include one correction and one deletion example. Keep the existing History and clipboard descriptions accurate.

Run `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check`, `cargo test --manifest-path src-tauri/Cargo.toml`, `npm run check`, `npm test`, and `npm run build`. In the desktop app, save `Hawfa` to `Haufe` and `uhm` to an empty value, restart Aywa, and make sure the rules remain. Dictate text with both matches and make sure History and the clipboard contain the same processed text. Check paste when the destination accepts it. Dictate text that the rules remove completely and make sure History, the clipboard, and the destination remain unchanged while the stated message appears.
