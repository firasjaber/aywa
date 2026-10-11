# Custom Recording Shortcut Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a Settings shortcut recorder that safely applies and persists a custom global hold-to-talk shortcut, while retaining the previous shortcut unless the candidate registers and saves successfully.

**Architecture:** Extend the existing `AppSettings` object and `state/settings.json` repository with a normalized `hotkey` string. Refactor the Rust hotkey handle into a small transactional controller that registers a candidate while routing events only for the old active ID, persists through a supplied callback, and switches the active ID only after persistence succeeds. Add a focused Svelte component and pure TypeScript capture helpers so keyboard-event behavior is independently testable.

**Tech Stack:** Rust 2021, Tauri 2, `global-hotkey` 0.6, Serde/JSON, Svelte 5, TypeScript, Vitest

---

## File Map

- Modify `src-tauri/src/settings.rs`: add the default/persisted hotkey and mutation API.
- Modify `src-tauri/src/hotkey.rs`: parse shortcuts and own transactional candidate activation.
- Modify `src-tauri/src/lib.rs`: orchestrate live changes through existing runtime/settings state.
- Modify `src-tauri/src/main.rs`: register the saved shortcut at startup and expose `set_hotkey`.
- Modify `src/lib/stores/appSettings.ts`: carry `hotkey` and invoke the new command.
- Create `src/lib/components/shortcutView.ts`: pure browser-event normalization and display helpers.
- Create `src/lib/components/shortcutView.test.ts`: unit tests for capture semantics.
- Create `src/lib/components/ShortcutPreference.svelte`: shortcut recorder UI and confirmed-state handling.
- Modify `src/lib/components/Settings.svelte`: replace the static keycaps with the recorder.
- Modify `README.md`, `docs/USER_GUIDE.md`, and `docs/TROUBLESHOOTING.md`: document configuration and conflict recovery.

### Task 1: Persist the shortcut in the existing settings file

**Files:**
- Modify: `src-tauri/src/settings.rs`

- [ ] **Step 1: Add failing default, compatibility, round-trip, and failed-save tests**

Add `DEFAULT_HOTKEY` to the test imports and these cases to `settings::tests`:

```rust
#[test]
fn defaults_use_ctrl_alt_space() {
    assert_eq!(AppSettings::default().hotkey, DEFAULT_HOTKEY);
}

#[test]
fn settings_without_hotkey_keep_history_and_use_default_shortcut() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("state/settings.json");
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(&path, r#"{"version":1,"saveHistory":false}"#).unwrap();

    let loaded = SettingsRepository::new(path).load().unwrap();
    assert!(!loaded.save_history);
    assert_eq!(loaded.hotkey, DEFAULT_HOTKEY);
}

#[test]
fn custom_hotkey_round_trips_in_the_shared_settings_file() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("state/settings.json");
    let repository = SettingsRepository::new(path.clone());
    let settings = AppSettings {
        hotkey: "control+shift+KeyD".to_string(),
        ..AppSettings::default()
    };

    repository.save(&settings).unwrap();
    assert_eq!(repository.load().unwrap(), settings);
    let contents = std::fs::read_to_string(path).unwrap();
    assert!(contents.contains(r#""saveHistory": true"#));
    assert!(contents.contains(r#""hotkey": "control+shift+KeyD""#));
}

#[test]
fn failed_hotkey_save_keeps_confirmed_in_memory_settings() {
    let directory = tempfile::tempdir().unwrap();
    let blocked = directory.path().join("state");
    std::fs::write(&blocked, "blocks directory creation").unwrap();
    let state = AppSettingsState::default();

    assert!(state
        .set_hotkey(
            &SettingsRepository::new(blocked.join("settings.json")),
            "control+shift+KeyD".to_string(),
        )
        .is_err());
    assert_eq!(state.current().unwrap().hotkey, DEFAULT_HOTKEY);
}
```

- [ ] **Step 2: Run the focused tests and confirm they fail**

Run:

```powershell
cargo test --manifest-path src-tauri/Cargo.toml settings::tests -- --nocapture
```

Expected: compilation fails because `AppSettings.hotkey`, `DEFAULT_HOTKEY`, and `set_hotkey` do not exist.

- [ ] **Step 3: Add the compatible settings field and mutation**

Add the default constant, field, default value, and mutation alongside `save_history`:

```rust
pub const SETTINGS_SCHEMA_VERSION: u32 = 1;
pub const DEFAULT_HOTKEY: &str = "control+alt+Space";

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
#[serde(default, rename_all = "camelCase")]
pub struct AppSettings {
    pub version: u32,
    pub save_history: bool,
    pub hotkey: String,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            version: SETTINGS_SCHEMA_VERSION,
            save_history: true,
            hotkey: DEFAULT_HOTKEY.to_string(),
        }
    }
}
```

Add to `AppSettingsState`:

```rust
pub fn set_hotkey(
    &self,
    repository: &SettingsRepository,
    hotkey: String,
) -> Result<AppSettings, String> {
    let mut current = self
        .current
        .lock()
        .map_err(|_| "app settings mutex poisoned".to_string())?;
    let mut next = current.clone();
    next.hotkey = hotkey;
    repository.save(&next)?;
    *current = next.clone();
    Ok(next)
}
```

Update existing `AppSettings` test literals only where the compiler requires the new field, using `..AppSettings::default()` so tests continue to express only their relevant differences.

- [ ] **Step 4: Run settings tests**

Run the command from Step 2.

Expected: all `settings::tests` pass, including loading the old History-only JSON without changing `saveHistory`.

- [ ] **Step 5: Commit the settings contract**

```powershell
git add src-tauri/src/settings.rs
git commit -m "feat: persist recording shortcut in app settings"
```

### Task 2: Make hotkey replacement transactional

**Files:**
- Modify: `src-tauri/src/hotkey.rs`

- [ ] **Step 1: Add failing parser and transaction tests**

Introduce a private `HotkeyRegistry` trait used by the production manager and a fake registry in tests. Add tests with these assertions:

```rust
#[test]
fn parser_requires_a_modifier_and_normalizes_the_chord() {
    assert_eq!(parse_shortcut("Ctrl + Shift + D").unwrap().to_string(), "shift+control+KeyD");
    assert!(parse_shortcut("D").unwrap_err().contains("modifier"));
}

#[test]
fn rejected_candidate_keeps_the_old_active_shortcut() {
    let (mut controller, registry) = test_controller();
    registry.fail_next_register();

    let result = controller.replace_with(parse_shortcut("Ctrl+Shift+D").unwrap(), |_| Ok(()));

    assert!(result.is_err());
    assert_eq!(controller.current().to_string(), DEFAULT_HOTKEY);
    assert_eq!(controller.active_id(), parse_shortcut(DEFAULT_HOTKEY).unwrap().id());
}

#[test]
fn failed_persistence_discards_candidate_and_keeps_old_active() {
    let (mut controller, registry) = test_controller();
    let candidate = parse_shortcut("Ctrl+Shift+D").unwrap();

    let result = controller.replace_with(candidate, |_| Err("disk full".to_string()));

    assert_eq!(result.unwrap_err(), "disk full");
    assert_eq!(controller.current().to_string(), DEFAULT_HOTKEY);
    assert_eq!(controller.active_id(), parse_shortcut(DEFAULT_HOTKEY).unwrap().id());
    assert!(registry.was_unregistered(candidate));
}

#[test]
fn successful_persistence_switches_active_id_before_cleaning_old_registration() {
    let (mut controller, registry) = test_controller();
    let old = controller.current();
    let candidate = parse_shortcut("Alt+KeyR").unwrap();

    controller.replace_with(candidate, |_| Ok(())).unwrap();

    assert_eq!(controller.current(), candidate);
    assert_eq!(controller.active_id(), candidate.id());
    assert!(registry.was_unregistered(old));
}
```

The test fake records registered/unregistered `HotKey` values in mutex-protected vectors and has a boolean consumed by `fail_next_register`. `test_controller` initializes the fake with the parsed default and an `Arc<AtomicU32>` containing the default ID.

- [ ] **Step 2: Confirm the hotkey tests fail**

```powershell
cargo test --manifest-path src-tauri/Cargo.toml hotkey::tests -- --nocapture
```

Expected: compilation fails because the parser, controller, registry abstraction, and helpers are absent.

- [ ] **Step 3: Implement parsing and the generic transaction controller**

Use the public fields exposed by `global_hotkey::hotkey::HotKey` and store the active event ID separately:

```rust
use std::sync::{
    atomic::{AtomicU32, Ordering},
    Arc,
};

use crate::{settings::DEFAULT_HOTKEY, start_recording, stop_recording};

pub fn parse_shortcut(value: &str) -> Result<HotKey, String> {
    let hotkey = value
        .parse::<HotKey>()
        .map_err(|_| "Choose a valid keyboard shortcut".to_string())?;
    if hotkey.mods.is_empty() {
        return Err("Shortcut must include Ctrl, Alt, Shift, or the Windows key".to_string());
    }
    Ok(hotkey)
}

trait HotkeyRegistry {
    fn register(&self, hotkey: HotKey) -> Result<(), String>;
    fn unregister(&self, hotkey: HotKey) -> Result<(), String>;
}

impl HotkeyRegistry for GlobalHotKeyManager {
    fn register(&self, hotkey: HotKey) -> Result<(), String> {
        GlobalHotKeyManager::register(self, hotkey).map_err(|_| {
            "That shortcut is unavailable. Choose another shortcut or close the app using it."
                .to_string()
        })
    }

    fn unregister(&self, hotkey: HotKey) -> Result<(), String> {
        GlobalHotKeyManager::unregister(self, hotkey).map_err(|error| error.to_string())
    }
}

struct HotkeyController<R> {
    registry: R,
    current: HotKey,
    active_id: Arc<AtomicU32>,
}

impl<R: HotkeyRegistry> HotkeyController<R> {
    fn replace_with<F>(&mut self, candidate: HotKey, persist: F) -> Result<Option<String>, String>
    where
        F: FnOnce(&str) -> Result<(), String>,
    {
        let normalized = candidate.to_string();
        if candidate == self.current {
            persist(&normalized)?;
            return Ok(None);
        }

        self.registry.register(candidate)?;
        if let Err(error) = persist(&normalized) {
            let _ = self.registry.unregister(candidate);
            return Err(error);
        }

        let previous = self.current;
        self.current = candidate;
        self.active_id.store(candidate.id(), Ordering::SeqCst);
        Ok(self.registry.unregister(previous).err())
    }
}
```

- [ ] **Step 4: Adapt the production handle and event handler**

Replace the fixed `_manager`/`_hotkey` fields with `controller: HotkeyController<GlobalHotKeyManager>`. Change registration to accept a shortcut string, parse it, register it, and capture `Arc<AtomicU32>` in the global event handler:

```rust
pub fn register<R: Runtime>(app: AppHandle<R>, shortcut: &str) -> Result<HotkeyHandle, String> {
    let hotkey = parse_shortcut(shortcut)?;
    let manager = GlobalHotKeyManager::new().map_err(|error| error.to_string())?;
    HotkeyRegistry::register(&manager, hotkey)?;
    let active_id = Arc::new(AtomicU32::new(hotkey.id()));
    let active_id_for_events = active_id.clone();
    let app_for_events = app.clone();

    GlobalHotKeyEvent::set_event_handler(Some(move |event: GlobalHotKeyEvent| {
        if event.id != active_id_for_events.load(Ordering::SeqCst) {
            return;
        }
        if event.state == HotKeyState::Pressed {
            let _ = start_recording(&app_for_events);
        } else if event.state == HotKeyState::Released {
            let _ = stop_recording(&app_for_events);
        }
    }));

    Ok(HotkeyHandle {
        controller: HotkeyController { registry: manager, current: hotkey, active_id },
    })
}
```

Expose a `replace_with` method on `HotkeyHandle` that parses a string and delegates to the controller. Preserve the existing documented `unsafe impl Send` and `unsafe impl Sync`; do not broaden unsafe code elsewhere.

- [ ] **Step 5: Run focused and full Rust tests**

```powershell
cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
cargo test --manifest-path src-tauri/Cargo.toml hotkey::tests -- --nocapture
cargo test --manifest-path src-tauri/Cargo.toml
```

Expected: formatting and all Rust tests pass.

- [ ] **Step 6: Commit the transactional hotkey controller**

```powershell
git add src-tauri/src/hotkey.rs
git commit -m "feat: switch recording shortcuts transactionally"
```

### Task 3: Wire startup registration and the live Tauri command

**Files:**
- Modify: `src-tauri/src/lib.rs`
- Modify: `src-tauri/src/main.rs`

- [ ] **Step 1: Add the settings-state mutation wrapper**

Add to `lib.rs` near `set_save_history`:

```rust
pub fn set_hotkey<R: Runtime>(
    app: &AppHandle<R>,
    state: &AppSettingsState,
    hotkey_value: String,
) -> Result<AppSettings, String> {
    if app.state::<AppStateManager>().current() != AppState::Ready {
        return Err("Wait for the current recording or transcription to finish".to_string());
    }

    let handles = app.state::<RuntimeHandles>();
    let mut handle = handles
        .hotkey
        .lock()
        .map_err(|_| "hotkey mutex poisoned".to_string())?;
    let handle = handle
        .as_mut()
        .ok_or_else(|| "The global shortcut is currently unavailable".to_string())?;
    let repository = SettingsRepository::new(settings_path(app)?);
    let mut confirmed = None;

    let cleanup_warning = handle.replace_with(&hotkey_value, |normalized| {
        let settings = state.set_hotkey(&repository, normalized.to_string())?;
        confirmed = Some(settings);
        Ok(())
    })?;
    if let Some(error) = cleanup_warning {
        diagnostics::log(format!("[hotkey] previous registration cleanup failed: {error}"));
    }
    confirmed.ok_or_else(|| "Shortcut settings were not confirmed".to_string())
}
```

Make `HotkeyHandle::replace_with` visible to the crate and return `Result<Option<String>, String>`.

- [ ] **Step 2: Add the Tauri command**

Add to `main.rs` beside `set_save_history`:

```rust
#[tauri::command]
fn set_hotkey(
    app: AppHandle,
    state: State<'_, AppSettingsState>,
    hotkey: String,
) -> Result<AppSettings, String> {
    aywa_lib::set_hotkey(&app, state.inner(), hotkey)
}
```

Include `set_hotkey` in `tauri::generate_handler!`.

- [ ] **Step 3: Register the persisted shortcut during startup with a safe default fallback**

Retain the `AppSettings` returned from `startup_app_settings`. Pass `settings.hotkey` into `hotkey::register`. If it fails and differs from `DEFAULT_HOTKEY`, try the default. After a successful fallback, persist the default through the same `AppSettingsState` and existing `SettingsRepository`; log sanitized failures. Store only a successfully created handle in `RuntimeHandles.hotkey`.

The startup branch must have these observable outcomes:

```text
saved shortcut registers -> saved shortcut active, JSON unchanged
saved shortcut rejected + default registers/saves -> default active and persisted
saved shortcut rejected + default rejected -> no active shortcut, app continues
```

- [ ] **Step 4: Compile and run Rust tests**

```powershell
cargo fmt --manifest-path src-tauri/Cargo.toml
cargo test --manifest-path src-tauri/Cargo.toml
```

Expected: compilation succeeds and all Rust tests pass.

- [ ] **Step 5: Commit backend orchestration**

```powershell
git add src-tauri/src/lib.rs src-tauri/src/main.rs
git commit -m "feat: apply saved shortcuts without restart"
```

### Task 4: Add frontend capture and display helpers

**Files:**
- Modify: `src/lib/stores/appSettings.ts`
- Create: `src/lib/components/shortcutView.ts`
- Create: `src/lib/components/shortcutView.test.ts`

- [ ] **Step 1: Write failing shortcut helper tests**

Create `shortcutView.test.ts`:

```ts
import { describe, expect, it } from 'vitest';
import { captureShortcut, shortcutLabels } from './shortcutView';

const key = (overrides: Partial<KeyboardEvent>) =>
	({ key: 'd', code: 'KeyD', ctrlKey: false, altKey: false, shiftKey: false, metaKey: false, ...overrides }) as KeyboardEvent;

describe('shortcut recorder helpers', () => {
	it('waits for a non-modifier key', () => {
		expect(captureShortcut(key({ key: 'Control', code: 'ControlLeft', ctrlKey: true }))).toEqual({ kind: 'wait' });
	});

	it('normalizes a modified key chord', () => {
		expect(captureShortcut(key({ ctrlKey: true, altKey: true }))).toEqual({
			kind: 'candidate',
			value: 'control+alt+KeyD'
		});
	});

	it('rejects an unmodified ordinary key', () => {
		expect(captureShortcut(key({}))).toEqual({ kind: 'invalid', message: 'Include at least one modifier key' });
	});

	it('cancels capture with Escape', () => {
		expect(captureShortcut(key({ key: 'Escape', code: 'Escape' }))).toEqual({ kind: 'cancel' });
	});

	it('renders backend-normalized labels', () => {
		expect(shortcutLabels('shift+control+KeyD')).toEqual(['Shift', 'Ctrl', 'D']);
		expect(shortcutLabels('super+Space')).toEqual(['Win', 'Space']);
	});
});
```

- [ ] **Step 2: Confirm the Vitest test fails**

```powershell
npm test -- src/lib/components/shortcutView.test.ts
```

Expected: FAIL because `shortcutView.ts` does not exist.

- [ ] **Step 3: Implement pure capture helpers**

Create `shortcutView.ts` with a discriminated result type, modifier-key set, fixed modifier order `shift`, `control`, `alt`, `super`, and these exports:

```ts
export type ShortcutCapture =
	| { kind: 'wait' }
	| { kind: 'cancel' }
	| { kind: 'invalid'; message: string }
	| { kind: 'candidate'; value: string };

const modifierCodes = new Set([
	'ControlLeft', 'ControlRight', 'AltLeft', 'AltRight',
	'ShiftLeft', 'ShiftRight', 'MetaLeft', 'MetaRight'
]);

export const captureShortcut = (event: KeyboardEvent): ShortcutCapture => {
	if (event.key === 'Escape') return { kind: 'cancel' };
	if (modifierCodes.has(event.code)) return { kind: 'wait' };
	const modifiers = [
		event.shiftKey ? 'shift' : null,
		event.ctrlKey ? 'control' : null,
		event.altKey ? 'alt' : null,
		event.metaKey ? 'super' : null
	].filter((value): value is string => value !== null);
	if (modifiers.length === 0) {
		return { kind: 'invalid', message: 'Include at least one modifier key' };
	}
	return { kind: 'candidate', value: [...modifiers, event.code].join('+') };
};
```

Implement `shortcutLabels(value)` by splitting `+`, mapping modifiers, and simplifying browser-code key names:

```ts
const modifierLabels: Record<string, string> = {
	shift: 'Shift',
	control: 'Ctrl',
	alt: 'Alt',
	super: 'Win'
};

const keyLabel = (value: string) => {
	if (value.startsWith('Key')) return value.slice(3);
	if (value.startsWith('Digit')) return value.slice(5);
	return value;
};

export const shortcutLabels = (value: string) =>
	value.split('+').map((part) => modifierLabels[part.toLowerCase()] ?? keyLabel(part));
```

- [ ] **Step 4: Extend the Tauri settings store**

Update `appSettings.ts`:

```ts
export type AppSettings = {
	version: number;
	saveHistory: boolean;
	hotkey: string;
};

export const setHotkey = (hotkey: string) =>
	invoke<AppSettings>('set_hotkey', { hotkey });
```

- [ ] **Step 5: Run frontend tests and type checks**

```powershell
npm test -- src/lib/components/shortcutView.test.ts
npm run check
```

Expected: helper tests and Svelte type checks pass.

- [ ] **Step 6: Commit frontend behavior primitives**

```powershell
git add src/lib/stores/appSettings.ts src/lib/components/shortcutView.ts src/lib/components/shortcutView.test.ts
git commit -m "feat: capture custom shortcut chords"
```

### Task 5: Build and integrate the shortcut recorder UI

**Files:**
- Create: `src/lib/components/ShortcutPreference.svelte`
- Modify: `src/lib/components/Settings.svelte`

- [ ] **Step 1: Create the focused recorder component**

The component imports `onMount`, `getAppSettings`, `setHotkey`, `captureShortcut`, and `shortcutLabels`. Its state is:

```ts
const DEFAULT_HOTKEY = 'control+alt+Space';
let confirmed = $state(DEFAULT_HOTKEY);
let loaded = $state(false);
let capturing = $state(false);
let saving = $state(false);
let error = $state<string | null>(null);
const labels = $derived(shortcutLabels(confirmed));
```

On mount, load settings and set `confirmed = settings.hotkey`. Implement one `apply(candidate)` function that leaves `confirmed` unchanged while awaiting `setHotkey`, updates it only from the returned settings, and displays the backend error on failure. Reset calls `apply(DEFAULT_HOTKEY)`.

Use a window handler only while `capturing`:

```ts
const handleKeydown = (event: KeyboardEvent) => {
	if (!capturing || saving) return;
	event.preventDefault();
	event.stopPropagation();
	const result = captureShortcut(event);
	if (result.kind === 'wait') return;
	if (result.kind === 'cancel') {
		capturing = false;
		error = null;
		return;
	}
	if (result.kind === 'invalid') {
		error = result.message;
		return;
	}
	capturing = false;
	void apply(result.value);
};
```

Render `<svelte:window onkeydown={handleKeydown} />`, confirmed keycaps, “Hold, speak, then release,” “Change shortcut,” and “Reset to default.” During capture replace the keycaps with “Press your shortcut…” and show “Press Escape to cancel.” Disable actions during initial loading/saving and expose errors through `role="alert"`.

- [ ] **Step 2: Replace the static Settings markup**

Import `ShortcutPreference` in `Settings.svelte`. Replace lines containing the static `.hotkey` block with:

```svelte
{#if appStatus.showHotkey}
	<ShortcutPreference />
{/if}
```

Move the `.hotkey` and `kbd` styling into the new component, extend it for action buttons/capture/error states, and remove the now-unused copies from `Settings.svelte`. Match existing dark surfaces, borders, font sizes, focus-visible outlines, and disabled opacity.

- [ ] **Step 3: Run frontend verification**

```powershell
npm test
npm run check
npm run build
```

Expected: all Vitest tests, Svelte checks, and the production frontend build pass.

- [ ] **Step 4: Commit the recorder UI**

```powershell
git add src/lib/components/ShortcutPreference.svelte src/lib/components/Settings.svelte
git commit -m "feat: add recording shortcut picker"
```

### Task 6: Update user documentation

**Files:**
- Modify: `README.md`
- Modify: `docs/USER_GUIDE.md`
- Modify: `docs/TROUBLESHOOTING.md`

- [ ] **Step 1: Document the configurable default**

Change the README feature bullet to:

```markdown
- Uses `Ctrl` + `Alt` + `Space` as the default hold-to-talk shortcut and lets you change it in Settings.
```

In the User Guide, keep the default in the dictation steps and add a “Change the dictation shortcut” subsection explaining: open App, choose Change shortcut, press a chord containing a modifier, use it immediately after confirmation, press Escape to cancel, and use Reset to default to restore `Ctrl+Alt+Space`.

Update Troubleshooting to tell users to verify the shortcut displayed on the App tab, choose a different chord if Windows reports it unavailable, and reset to default when recovery is needed.

- [ ] **Step 2: Check current-facing text for stale fixed-shortcut claims**

```powershell
rg -n "uses.*Ctrl|shortcut is|Ctrl.*Alt.*Space" README.md docs/USER_GUIDE.md docs/TROUBLESHOOTING.md src/lib/components
```

Expected: every current-facing match either calls the chord the default, provides dictation instructions using the default, or describes reset behavior; the Settings component contains no hard-coded active shortcut display.

- [ ] **Step 3: Commit documentation**

```powershell
git add README.md docs/USER_GUIDE.md docs/TROUBLESHOOTING.md
git commit -m "docs: explain custom recording shortcuts"
```

### Task 7: Full verification and Windows smoke test

**Files:**
- Verify all files changed above

- [ ] **Step 1: Run automated verification**

```powershell
cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
cargo test --manifest-path src-tauri/Cargo.toml
npm test
npm run check
npm run build
git diff --check abf1625..HEAD
```

Expected: every command exits successfully and `git diff --check` emits no whitespace errors.

- [ ] **Step 2: Perform the Windows behavior smoke test**

Run Aywa and verify in order:

1. Existing `Ctrl+Alt+Space` still records on press and transcribes on release.
2. Change to `Ctrl+Shift+D`; before backend confirmation the old shortcut remains displayed and active.
3. After confirmation, only `Ctrl+Shift+D` triggers recording.
4. Close and reopen Aywa; `Ctrl+Shift+D` remains displayed and active.
5. Attempt an unmodified `D`; the UI rejects it and `Ctrl+Shift+D` remains active.
6. Attempt a chord owned by another app; Aywa reports the conflict and `Ctrl+Shift+D` remains active and remains in `state/settings.json`.
7. Make `state/settings.json` temporarily unwritable, attempt another valid chord, and confirm the old shortcut remains active and the file is unchanged; restore permissions immediately afterward.
8. Start capture and press Escape; no setting changes.
9. Reset to default; `Ctrl+Alt+Space` becomes active immediately and persists after restart.
10. Confirm the same JSON object still contains the unchanged `saveHistory` value and the confirmed `hotkey` value.

- [ ] **Step 3: Inspect final repository state**

```powershell
git status --short
git log -8 --oneline
```

Expected: the worktree is clean and the feature is represented by focused commits for settings, backend switching, frontend capture, UI, and docs.
