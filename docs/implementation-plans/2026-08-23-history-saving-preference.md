# History-Saving Preference Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a persisted, default-on setting that lets users stop future transcription text from being stored in History without changing clipboard, paste, or existing History data.

**Architecture:** A dedicated Rust settings module owns the versioned `<app-local-data>/state/settings.json` document and an in-memory `AppSettingsState`. The transcription path reads that state and delegates conditional persistence to `HistoryRepository::save_if_enabled`; a focused Svelte component uses typed Tauri commands and a pure view-state helper to load, update, and roll back the checkbox.

**Tech Stack:** Rust 2021, Tauri 2 commands and managed state, Serde/JSON, Svelte 5 runes, TypeScript, Vitest, Cargo tests.

---

## File map

- Create `src-tauri/src/settings.rs`: settings schema, disk repository, managed state, and unit tests.
- Modify `src-tauri/Cargo.toml`: add the Windows file-replacement API used for atomic settings updates.
- Modify `src-tauri/src/history.rs`: add the conditional History-save boundary and its focused tests.
- Modify `src-tauri/src/lib.rs`: expose settings functions and consult the setting in the successful transcription path.
- Modify `src-tauri/src/main.rs`: manage settings state, load it at startup, and register get/set commands.
- Create `src/lib/stores/appSettings.ts`: typed frontend wrappers around the settings commands.
- Create `src/lib/components/appSettingsView.ts`: pure loading/saving/rollback state transitions and user-facing copy.
- Create `src/lib/components/appSettingsView.test.ts`: frontend state-transition tests.
- Create `src/lib/components/HistoryPreference.svelte`: the compact, self-contained preference UI.
- Modify `src/lib/components/Settings.svelte`: place the new component on the App tab.
- Modify `docs/USER_GUIDE.md`: explain the setting and its effect on future versus existing History.

### Task 1: Persist extensible app settings

**Files:**
- Create: `src-tauri/src/settings.rs`
- Modify: `src-tauri/Cargo.toml:24-38`
- Modify: `src-tauri/src/lib.rs:1-14`
- Test: `src-tauri/src/settings.rs`

- [ ] **Step 1: Write the failing settings tests**

Create `src-tauri/src/settings.rs` with the public schema declarations and this test module first:

```rust
use std::{
    fs,
    os::windows::ffi::OsStrExt,
    path::{Path, PathBuf},
    sync::Mutex,
};

use windows_sys::Win32::Storage::FileSystem::{
    MoveFileExW, MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH,
};

pub const SETTINGS_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
#[serde(default, rename_all = "camelCase")]
pub struct AppSettings {
    pub version: u32,
    pub save_history: bool,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self { version: SETTINGS_SCHEMA_VERSION, save_history: true }
    }
}

pub struct SettingsRepository { path: PathBuf }
pub struct AppSettingsState { current: Mutex<AppSettings> }

#[cfg(test)]
mod tests {
    use super::{AppSettings, AppSettingsState, SettingsRepository, SETTINGS_SCHEMA_VERSION};

    #[test]
    fn defaults_enable_history_saving() {
        let settings = AppSettings::default();
        assert_eq!(settings.version, SETTINGS_SCHEMA_VERSION);
        assert!(settings.save_history);
    }

    #[test]
    fn missing_file_loads_defaults_without_creating_a_file() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("state/settings.json");
        let repository = SettingsRepository::new(path.clone());
        assert_eq!(repository.load().unwrap(), AppSettings::default());
        assert!(!path.exists());
    }

    #[test]
    fn settings_round_trip_and_replace_the_previous_value() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("state/settings.json");
        let repository = SettingsRepository::new(path.clone());
        let disabled = AppSettings { save_history: false, ..AppSettings::default() };
        repository.save(&disabled).unwrap();
        assert_eq!(repository.load().unwrap(), disabled);
        repository.save(&AppSettings::default()).unwrap();
        assert_eq!(repository.load().unwrap(), AppSettings::default());
        assert!(!path.with_extension("json.tmp").exists());
    }

    #[test]
    fn invalid_json_and_unknown_versions_are_rejected() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("state/settings.json");
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, "not json").unwrap();
        let repository = SettingsRepository::new(path.clone());
        assert!(repository.load().unwrap_err().contains("read app settings"));
        std::fs::write(&path, r#"{"version":2,"saveHistory":false}"#).unwrap();
        assert!(repository.load().unwrap_err().contains("unsupported settings version"));
    }

    #[test]
    fn failed_save_does_not_change_confirmed_in_memory_state() {
        let directory = tempfile::tempdir().unwrap();
        let unwritable_path = directory.path().join("state");
        std::fs::write(&unwritable_path, "blocks directory creation").unwrap();
        let repository = SettingsRepository::new(unwritable_path.join("settings.json"));
        let state = AppSettingsState::default();
        assert!(state.set_save_history(&repository, false).is_err());
        assert!(state.current().unwrap().save_history);
    }
}
```

Add `pub mod settings;` beside the other module declarations in `src-tauri/src/lib.rs`.

- [ ] **Step 2: Run the focused tests to verify they fail**

Run:

```powershell
cargo test --manifest-path src-tauri/Cargo.toml settings::tests -- --nocapture
```

Expected: compilation fails because `SettingsRepository` and `AppSettingsState` do not yet implement the methods exercised by the tests.

- [ ] **Step 3: Add the Windows atomic-replacement dependency**

Add this direct dependency under `[dependencies]` in `src-tauri/Cargo.toml`:

```toml
windows-sys = { version = "0.59", features = ["Win32_Storage_FileSystem"] }
```

Run:

```powershell
cargo check --manifest-path src-tauri/Cargo.toml
```

Expected: Cargo resolves the Windows API crate and the non-test library check passes; the failing unit tests remain red until the next step.

- [ ] **Step 4: Implement the repository and managed state**

Add these implementations above the test module in `src-tauri/src/settings.rs`:

```rust
impl SettingsRepository {
    pub fn new(path: PathBuf) -> Self { Self { path } }

    pub fn load(&self) -> Result<AppSettings, String> {
        let contents = match fs::read(&self.path) {
            Ok(contents) => contents,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Ok(AppSettings::default());
            }
            Err(error) => return Err(format!("failed to read app settings: {error}")),
        };
        let settings: AppSettings = serde_json::from_slice(&contents)
            .map_err(|error| format!("failed to read app settings: {error}"))?;
        if settings.version != SETTINGS_SCHEMA_VERSION {
            return Err(format!("unsupported settings version: {}", settings.version));
        }
        Ok(settings)
    }

    pub fn save(&self, settings: &AppSettings) -> Result<(), String> {
        let parent = self.path.parent()
            .ok_or_else(|| "settings path has no parent directory".to_string())?;
        fs::create_dir_all(parent)
            .map_err(|error| format!("failed to create settings directory: {error}"))?;
        let contents = serde_json::to_vec_pretty(settings)
            .map_err(|error| format!("failed to encode app settings: {error}"))?;
        atomic_write(&self.path, &contents)
    }
}

impl Default for AppSettingsState {
    fn default() -> Self {
        Self { current: Mutex::new(AppSettings::default()) }
    }
}

impl AppSettingsState {
    pub fn current(&self) -> Result<AppSettings, String> {
        self.current.lock()
            .map_err(|_| "app settings mutex poisoned".to_string())
            .map(|settings| settings.clone())
    }

    pub fn replace(&self, settings: AppSettings) -> Result<AppSettings, String> {
        let mut current = self.current.lock()
            .map_err(|_| "app settings mutex poisoned".to_string())?;
        *current = settings.clone();
        Ok(settings)
    }

    pub fn set_save_history(
        &self,
        repository: &SettingsRepository,
        save_history: bool,
    ) -> Result<AppSettings, String> {
        let mut next = self.current()?;
        next.save_history = save_history;
        repository.save(&next)?;
        self.replace(next)
    }
}

fn atomic_write(path: &Path, contents: &[u8]) -> Result<(), String> {
    let temporary = path.with_extension("json.tmp");
    fs::write(&temporary, contents)
        .map_err(|error| format!("failed to write app settings: {error}"))?;
    let source = temporary.as_os_str().encode_wide().chain(Some(0)).collect::<Vec<_>>();
    let destination = path.as_os_str().encode_wide().chain(Some(0)).collect::<Vec<_>>();
    let replaced = unsafe {
        MoveFileExW(
            source.as_ptr(),
            destination.as_ptr(),
            MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
        )
    };
    if replaced == 0 {
        let error = std::io::Error::last_os_error();
        let _ = fs::remove_file(&temporary);
        return Err(format!("failed to finalize app settings: {error}"));
    }
    Ok(())
}
```

- [ ] **Step 5: Run the settings tests and formatting**

Run:

```powershell
cargo fmt --manifest-path src-tauri/Cargo.toml
cargo test --manifest-path src-tauri/Cargo.toml settings::tests -- --nocapture
```

Expected: Rust formatting completes and all five settings tests pass.

- [ ] **Step 6: Commit the settings foundation**

```powershell
git add src-tauri/Cargo.toml src-tauri/Cargo.lock src-tauri/src/settings.rs src-tauri/src/lib.rs
git commit -m "feat: add persisted app settings"
```

### Task 2: Wire settings into Tauri and conditional History persistence

**Files:**
- Modify: `src-tauri/src/history.rs:35-71,190-260`
- Modify: `src-tauri/src/lib.rs:15-205`
- Modify: `src-tauri/src/main.rs:10-282`
- Test: `src-tauri/src/history.rs`

- [ ] **Step 1: Write the failing conditional-save tests**

Add these tests to `src-tauri/src/history.rs`'s existing test module:

```rust
#[test]
fn disabled_history_does_not_write_metadata_or_transcript_files() {
    let directory = tempfile::tempdir().unwrap();
    let repository = HistoryRepository::new(directory.path().join("history"));
    let result = repository.save_if_enabled(false, "private text", 500, None).unwrap();
    assert_eq!(result, None);
    assert!(repository.list(0, 50).unwrap().entries.is_empty());
    assert!(!directory.path().join("history/transcripts").exists());
}

#[test]
fn enabled_history_uses_the_existing_save_path() {
    let directory = tempfile::tempdir().unwrap();
    let repository = HistoryRepository::new(directory.path().join("history"));
    let saved = repository
        .save_if_enabled(true, "retained text", 500, None)
        .unwrap()
        .unwrap();
    assert_eq!(repository.read(&saved.id).unwrap().text, "retained text");
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run:

```powershell
cargo test --manifest-path src-tauri/Cargo.toml history::tests -- --nocapture
```

Expected: compilation fails because `save_if_enabled` does not exist.

- [ ] **Step 3: Add the conditional repository method**

Add this public method immediately before `HistoryRepository::save`:

```rust
pub fn save_if_enabled(
    &self,
    enabled: bool,
    text: &str,
    duration_ms: u64,
    language: Option<String>,
) -> Result<Option<HistoryEntryMetadata>, String> {
    if !enabled {
        return Ok(None);
    }
    self.save(text, duration_ms, language).map(Some)
}
```

- [ ] **Step 4: Expose startup, read, and update functions in the library**

Add `settings::{AppSettings, AppSettingsState, SettingsRepository}` to the `use crate::{...}` block in `src-tauri/src/lib.rs`, then add:

```rust
pub fn settings_path<R: Runtime>(app: &AppHandle<R>) -> Result<PathBuf, String> {
    Ok(app_data_dir(app)?.join("state").join("settings.json"))
}

pub fn startup_app_settings<R: Runtime>(
    app: &AppHandle<R>,
    state: &AppSettingsState,
) -> AppSettings {
    let loaded = settings_path(app)
        .and_then(|path| SettingsRepository::new(path).load());
    match loaded {
        Ok(settings) => state.replace(settings.clone()).unwrap_or(settings),
        Err(error) => {
            diagnostics::log(format!("[settings] load failed; using defaults: {error}"));
            let defaults = AppSettings::default();
            state.replace(defaults.clone()).unwrap_or(defaults)
        }
    }
}

pub fn get_app_settings(state: &AppSettingsState) -> Result<AppSettings, String> {
    state.current()
}

pub fn set_save_history<R: Runtime>(
    app: &AppHandle<R>,
    state: &AppSettingsState,
    save_history: bool,
) -> Result<AppSettings, String> {
    state.set_save_history(
        &SettingsRepository::new(settings_path(app)?),
        save_history,
    )
}
```

Replace the unconditional History save in the successful branch of `stop_recording` with:

```rust
let save_history = app
    .state::<AppSettingsState>()
    .current()
    .map(|settings| settings.save_history)
    .unwrap_or_else(|error| {
        diagnostics::log(format!("[settings] read failed; preserving default: {error}"));
        true
    });
if let Err(error) = history_dir(app).and_then(|directory| {
    history::HistoryRepository::new(directory)
        .save_if_enabled(save_history, &transcription.text, result.duration_ms, None)
        .map(|_| ())
}) {
    diagnostics::log(format!("[history] save failed: {error}"));
}
```

Keep `clipboard::copy_text` and paste simulation immediately after this block.

- [ ] **Step 5: Register managed state and commands**

In `src-tauri/src/main.rs`, import `settings::{AppSettings, AppSettingsState}` and add these commands beside the History commands:

```rust
#[tauri::command]
fn get_app_settings(state: State<'_, AppSettingsState>) -> Result<AppSettings, String> {
    aywa_lib::get_app_settings(state.inner())
}

#[tauri::command]
fn set_save_history(
    app: AppHandle,
    state: State<'_, AppSettingsState>,
    save_history: bool,
) -> Result<AppSettings, String> {
    aywa_lib::set_save_history(&app, state.inner(), save_history)
}
```

Add `.manage(AppSettingsState::default())` next to the existing managed states. In `setup`, after diagnostics initialization, load the file into managed state:

```rust
let settings_state: State<'_, AppSettingsState> = app.state();
aywa_lib::startup_app_settings(&app.handle(), settings_state.inner());
```

Add `get_app_settings` and `set_save_history` to `tauri::generate_handler!`.

- [ ] **Step 6: Run focused and complete Rust verification**

Run:

```powershell
cargo fmt --manifest-path src-tauri/Cargo.toml
cargo test --manifest-path src-tauri/Cargo.toml history::tests -- --nocapture
cargo test --manifest-path src-tauri/Cargo.toml settings::tests -- --nocapture
cargo test --manifest-path src-tauri/Cargo.toml
```

Expected: conditional-save tests and the complete Rust suite pass. Confirm the existing clipboard/paste lines remain outside the setting guard with:

```powershell
rg -n -A 20 "save_if_enabled" src-tauri/src/lib.rs
```

Expected: `clipboard::copy_text` and `paste_clipboard` appear after the conditional History block.

- [ ] **Step 7: Commit backend integration**

```powershell
git add src-tauri/src/history.rs src-tauri/src/lib.rs src-tauri/src/main.rs
git commit -m "feat: honor history saving preference"
```

### Task 3: Add a testable frontend preference state model

**Files:**
- Create: `src/lib/stores/appSettings.ts`
- Create: `src/lib/components/appSettingsView.ts`
- Create: `src/lib/components/appSettingsView.test.ts`

- [ ] **Step 1: Write the failing view-state tests**

Create `src/lib/components/appSettingsView.test.ts`:

```ts
import { describe, expect, it } from 'vitest';
import {
	HISTORY_PREFERENCE_DESCRIPTION,
	beginHistoryPreferenceSave,
	historyPreferenceLoadFailed,
	historyPreferenceLoaded,
	historyPreferenceSaveFailed,
	historyPreferenceSaved,
	initialHistoryPreferenceState
} from './appSettingsView';

describe('history preference view state', () => {
	it('starts disabled until the persisted setting loads', () => {
		const state = initialHistoryPreferenceState();
		expect(state).toEqual({ loaded: false, saving: false, confirmed: true, displayed: true, error: null });
	});

	it('loads the persisted value', () => {
		expect(historyPreferenceLoaded(false)).toMatchObject({ loaded: true, confirmed: false, displayed: false });
	});

	it('disables the control while saving and confirms success', () => {
		const saving = beginHistoryPreferenceSave(historyPreferenceLoaded(true), false);
		expect(saving).toMatchObject({ saving: true, confirmed: true, displayed: false });
		expect(historyPreferenceSaved(saving, false)).toMatchObject({ saving: false, confirmed: false, displayed: false });
	});

	it('reverts to the last confirmed value after a failed save', () => {
		const saving = beginHistoryPreferenceSave(historyPreferenceLoaded(true), false);
		expect(historyPreferenceSaveFailed(saving, 'disk full')).toMatchObject({
			saving: false, confirmed: true, displayed: true, error: 'disk full'
		});
	});

	it('reports load failure while preserving the safe displayed default', () => {
		expect(historyPreferenceLoadFailed('unavailable')).toMatchObject({ loaded: false, displayed: true, error: 'unavailable' });
	});

	it('explains that disabling applies only to future transcripts', () => {
		expect(HISTORY_PREFERENCE_DESCRIPTION).toBe(
			'When off, future transcriptions are not saved. Existing History remains until you clear it.'
		);
	});
});
```

- [ ] **Step 2: Run the focused test to verify it fails**

Run:

```powershell
npx vitest run src/lib/components/appSettingsView.test.ts
```

Expected: FAIL because `appSettingsView.ts` does not exist.

- [ ] **Step 3: Implement the typed command wrappers and state transitions**

Create `src/lib/stores/appSettings.ts`:

```ts
import { invoke } from '@tauri-apps/api/core';

export type AppSettings = { version: number; saveHistory: boolean };

export const getAppSettings = () => invoke<AppSettings>('get_app_settings');
export const setSaveHistory = (saveHistory: boolean) =>
	invoke<AppSettings>('set_save_history', { saveHistory });
```

Create `src/lib/components/appSettingsView.ts`:

```ts
export const HISTORY_PREFERENCE_DESCRIPTION =
	'When off, future transcriptions are not saved. Existing History remains until you clear it.';

export type HistoryPreferenceState = {
	loaded: boolean;
	saving: boolean;
	confirmed: boolean;
	displayed: boolean;
	error: string | null;
};

export const initialHistoryPreferenceState = (): HistoryPreferenceState => ({
	loaded: false, saving: false, confirmed: true, displayed: true, error: null
});

export const historyPreferenceLoaded = (value: boolean): HistoryPreferenceState => ({
	loaded: true, saving: false, confirmed: value, displayed: value, error: null
});

export const historyPreferenceLoadFailed = (error: string): HistoryPreferenceState => ({
	...initialHistoryPreferenceState(), error
});

export const beginHistoryPreferenceSave = (
	state: HistoryPreferenceState,
	displayed: boolean
): HistoryPreferenceState => ({ ...state, saving: true, displayed, error: null });

export const historyPreferenceSaved = (
	state: HistoryPreferenceState,
	confirmed: boolean
): HistoryPreferenceState => ({ ...state, saving: false, confirmed, displayed: confirmed, error: null });

export const historyPreferenceSaveFailed = (
	state: HistoryPreferenceState,
	error: string
): HistoryPreferenceState => ({ ...state, saving: false, displayed: state.confirmed, error });
```

- [ ] **Step 4: Run frontend tests and type checks**

Run:

```powershell
npx vitest run src/lib/components/appSettingsView.test.ts
npm run check
```

Expected: the six focused tests pass and Svelte/TypeScript checks pass.

- [ ] **Step 5: Commit the frontend preference model**

```powershell
git add src/lib/stores/appSettings.ts src/lib/components/appSettingsView.ts src/lib/components/appSettingsView.test.ts
git commit -m "feat: add history preference state model"
```

### Task 4: Render and persist the App-tab control

**Files:**
- Create: `src/lib/components/HistoryPreference.svelte`
- Modify: `src/lib/components/Settings.svelte:1-8,320-360`

- [ ] **Step 1: Create the focused Svelte preference component**

Create `src/lib/components/HistoryPreference.svelte`:

```svelte
<script lang="ts">
	import { onMount } from 'svelte';
	import { getAppSettings, setSaveHistory } from '$lib/stores/appSettings';
	import {
		HISTORY_PREFERENCE_DESCRIPTION,
		beginHistoryPreferenceSave,
		historyPreferenceLoadFailed,
		historyPreferenceLoaded,
		historyPreferenceSaveFailed,
		historyPreferenceSaved,
		initialHistoryPreferenceState
	} from './appSettingsView';

	let state = $state(initialHistoryPreferenceState());
	const errorText = (error: unknown) => error instanceof Error ? error.message : String(error);

	onMount(() => {
		getAppSettings()
			.then((settings) => { state = historyPreferenceLoaded(settings.saveHistory); })
			.catch((error) => { state = historyPreferenceLoadFailed(errorText(error)); });
	});

	const changePreference = async (event: Event) => {
		const saveHistory = (event.currentTarget as HTMLInputElement).checked;
		state = beginHistoryPreferenceSave(state, saveHistory);
		try {
			const settings = await setSaveHistory(saveHistory);
			state = historyPreferenceSaved(state, settings.saveHistory);
		} catch (error) {
			state = historyPreferenceSaveFailed(state, errorText(error));
		}
	};
</script>

<section class="history-preference" aria-labelledby="history-preference-heading">
	<div>
		<h2 id="history-preference-heading">History</h2>
		<p>{HISTORY_PREFERENCE_DESCRIPTION}</p>
	</div>
	<label>
		<input
			type="checkbox"
			checked={state.displayed}
			disabled={!state.loaded || state.saving}
			onchange={changePreference}
		/>
		<span>Save transcriptions to History</span>
	</label>
	{#if state.error}
		<p class="error" role="alert">The History preference could not be loaded or saved. Try again.</p>
	{/if}
</section>

<style>
	.history-preference { display: grid; gap: .75rem; margin-top: 1.1rem; border: 1px solid #50534f; border-radius: .75rem; padding: .85rem; background: #1b1c1b; }
	h2, p { margin: 0; }
	h2 { font-size: 14px; }
	p { margin-top: .3rem; color: #bfc1bc; font-size: 12px; line-height: 1.45; }
	label { display: flex; align-items: center; gap: .55rem; color: #f4f4f0; font-size: 13px; font-weight: 650; cursor: pointer; }
	input { width: 1rem; height: 1rem; accent-color: #c7f164; }
	input:disabled { cursor: default; opacity: .6; }
	.error { color: #ffb7b7; }
</style>
```

- [ ] **Step 2: Place the component on the App tab**

Add this import near the existing `History` import in `src/lib/components/Settings.svelte`:

```ts
import HistoryPreference from './HistoryPreference.svelte';
```

Render the control immediately after the Models section and before `actionError`:

```svelte
<HistoryPreference />
```

- [ ] **Step 3: Verify the frontend build and tests**

Run:

```powershell
npm test
npm run check
npm run build
```

Expected: all Vitest tests pass, Svelte reports zero errors, and the production bundle builds.

- [ ] **Step 4: Commit the App-tab UI**

```powershell
git add src/lib/components/HistoryPreference.svelte src/lib/components/Settings.svelte
git commit -m "feat: add history saving control"
```

### Task 5: Document and verify the complete behavior

**Files:**
- Modify: `docs/USER_GUIDE.md:72-79`

- [ ] **Step 1: Update the user guide**

Replace the first paragraph under `## Review transcription history` with:

```markdown
Every successful transcription is saved automatically in the local **History** tab by default. To stop saving future transcription text, open the **App** tab and turn off **Save transcriptions to History**. Dictation still copies and attempts to paste text while this setting is off. Existing History remains until you delete individual entries or use **Clear all history**.

History retains the text, time, and recording length only; the temporary WAV recording is still deleted after transcription. You can open an entry to read and copy it, delete individual entries, or permanently clear all History. Aywa does not upload recordings or transcription history.
```

- [ ] **Step 2: Run the complete automated verification**

Run:

```powershell
npm test
npm run check
npm run build
cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
cargo test --manifest-path src-tauri/Cargo.toml
git diff --check
```

Expected: every command exits successfully and `git diff --check` prints no errors.

- [ ] **Step 3: Perform the Windows smoke check**

Run the app with:

```powershell
npm run tauri:dev:managed-python
```

Verify in order:

1. App tab initially shows **Save transcriptions to History** checked.
2. Turn it off, close the Settings window, reopen it, and confirm it remains off.
3. Dictate once and confirm text is copied/pasted but no new History row appears.
4. Confirm History entries created before disabling the setting remain readable.
5. Turn the setting on, dictate once, and confirm a new History row appears.
6. Quit and reopen Aywa; confirm the last selected setting survives restart.
7. Inspect `<app-local-data>/state/settings.json` and confirm it contains version `1` and the selected `saveHistory` value.

- [ ] **Step 4: Commit documentation and final verification state**

```powershell
git add docs/USER_GUIDE.md
git commit -m "docs: explain history saving preference"
git status --short
```

Expected: the documentation commit succeeds and the worktree is clean.
