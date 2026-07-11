# Compact App and Debug UI Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the developer-oriented settings page with the approved compact, dark App/Debug interface while preserving the native Windows title bar and existing model/runtime behavior.

**Architecture:** Keep the existing `ModelManagerSnapshot` as the source of truth, extend it with a basic operating-system label, and add one Tauri command that reveals the diagnostics log in Windows Explorer. Put presentation mapping in a small pure TypeScript module, while `Settings.svelte` owns tab selection, simulated download progress, activation feedback, and support-action errors.

**Tech Stack:** Svelte 5, TypeScript, Tauri 2, Rust, CSS, Vite

**Repository constraint:** The worktree contains unrelated user changes. Do not reset, stage, or commit any files. Commit steps are intentionally omitted despite the standard planning template.

---

## File structure

- Create `src/lib/components/settingsView.ts`: pure labels and state-selection helpers used by the settings component.
- Modify `src/lib/components/Settings.svelte`: compact App/Debug interface, local progress/activation state, and scoped visual styling.
- Modify `src/lib/stores/modelManager.ts`: add the OS field and expose the log-opening command.
- Modify `src-tauri/src/diagnostics.rs`: return a stable OS label and reveal the diagnostics log in Explorer.
- Modify `src-tauri/src/model_manager/service.rs`: serialize the OS label with the existing settings snapshot.
- Modify `src-tauri/src/lib.rs`: expose the diagnostics-log operation to the Tauri command layer.
- Modify `src-tauri/src/main.rs`: register `open_diagnostics_log`.
- Modify `src-tauri/tauri.conf.json`: set compact default and minimum dimensions while retaining native decorations and resizing.
- Modify `src/app.css`: switch the settings root to the selected dark native visual baseline.

### Task 1: Add basic system identity and Windows log reveal

**Files:**
- Modify: `src-tauri/src/diagnostics.rs`
- Modify: `src-tauri/src/model_manager/service.rs`
- Modify: `src-tauri/src/lib.rs`
- Modify: `src-tauri/src/main.rs`
- Test: inline unit tests in `src-tauri/src/diagnostics.rs`

- [ ] **Step 1: Add failing diagnostics helper tests**

Append a test module that fixes the user-facing platform label and Explorer argument format:

```rust
#[cfg(test)]
mod ui_support_tests {
    use std::path::Path;

    use super::{explorer_select_arg, operating_system_label};

    #[test]
    fn operating_system_label_is_human_readable() {
        let label = operating_system_label();
        assert!(!label.is_empty());
        assert_ne!(label, "windows");
    }

    #[test]
    fn explorer_select_arg_targets_the_log_file() {
        let path = Path::new(r"C:\Users\Test User\AppData\Local\ok\logs\ok-debug.log");
        assert_eq!(
            explorer_select_arg(path),
            r"/select,C:\Users\Test User\AppData\Local\ok\logs\ok-debug.log"
        );
    }
}
```

- [ ] **Step 2: Run the tests and confirm the missing helpers fail compilation**

Run:

```bash
cargo test --manifest-path src-tauri/Cargo.toml ui_support_tests
```

Expected: compilation fails because `operating_system_label` and `explorer_select_arg` do not exist.

- [ ] **Step 3: Implement the pure helpers and log reveal operation**

Add to `diagnostics.rs`:

```rust
pub fn operating_system_label() -> String {
    match std::env::consts::OS {
        "windows" => "Windows".to_string(),
        "macos" => "macOS".to_string(),
        "linux" => "Linux".to_string(),
        other => other.to_string(),
    }
}

pub fn explorer_select_arg(path: &Path) -> String {
    format!("/select,{}", path.display())
}

pub fn open_log_in_file_manager<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
) -> Result<(), String> {
    let path = log_path_for_app(app)
        .ok_or_else(|| "The diagnostics log is not available yet.".to_string())?;

    if !path.exists() {
        return Err("The diagnostics log has not been created yet.".to_string());
    }

    #[cfg(target_os = "windows")]
    {
        Command::new("explorer.exe")
            .arg(explorer_select_arg(&path))
            .spawn()
            .map_err(|error| format!("Could not open Windows Explorer: {error}"))?;
        return Ok(());
    }

    #[cfg(not(target_os = "windows"))]
    Err("Opening the diagnostics log is currently supported on Windows only.".to_string())
}
```

`Command` is already imported in this module. Do not add a shell invocation or pass log contents to the frontend.

- [ ] **Step 4: Add the OS label to the settings snapshot**

Extend `ModelManagerSnapshot` in `model_manager/service.rs`:

```rust
pub struct ModelManagerSnapshot {
    pub selected_model_id: ModelId,
    pub loaded_model_id: Option<ModelId>,
    pub operating_system: String,
    pub runtime_preflight: RuntimePreflight,
    pub worker_cuda_status: WorkerCudaStatus,
    pub worker_runtime_bootstrap: WorkerRuntimeBootstrap,
    pub runtime_profile: RuntimeProfileSnapshot,
    pub diagnostics_log_path: Option<String>,
    pub ready_for_transcription: bool,
    pub models: Vec<ModelRow>,
}
```

Populate it inside `snapshot_from_registry`:

```rust
ModelManagerSnapshot {
    selected_model_id: registry.selected_model_id,
    loaded_model_id: registry.loaded_model_id,
    operating_system: diagnostics::operating_system_label(),
    ready_for_transcription: registry.loaded_model_id.is_some() && runtime_ready,
    runtime_preflight,
    worker_cuda_status,
    worker_runtime_bootstrap,
    runtime_profile,
    diagnostics_log_path,
    models,
}
```

- [ ] **Step 5: Expose and register the Tauri command**

Add to `src-tauri/src/lib.rs`:

```rust
pub fn open_diagnostics_log<R: Runtime>(app: &AppHandle<R>) -> Result<(), String> {
    diagnostics::open_log_in_file_manager(app)
}
```

Add to `src-tauri/src/main.rs`:

```rust
#[tauri::command]
fn open_diagnostics_log(app: AppHandle) -> Result<(), String> {
    ok_lib::open_diagnostics_log(&app)
}
```

Register `open_diagnostics_log` in `tauri::generate_handler![...]` next to `list_models`.

- [ ] **Step 6: Run focused and full Rust tests**

Run:

```bash
cargo test --manifest-path src-tauri/Cargo.toml ui_support_tests
cargo test --manifest-path src-tauri/Cargo.toml
```

Expected: all tests pass and the snapshot serialization compiles with `operatingSystem` through the existing camel-case serializer.

### Task 2: Add the frontend data contract and pure presentation mapping

**Files:**
- Create: `src/lib/components/settingsView.ts`
- Modify: `src/lib/stores/modelManager.ts`

- [ ] **Step 1: Extend the TypeScript snapshot and command API**

Add `operatingSystem` to `ModelManagerSnapshot`, initialize it, and expose the command:

```ts
export type ModelManagerSnapshot = {
	selectedModelId: ModelId;
	loadedModelId: ModelId | null;
	operatingSystem: string;
	runtimePreflight: RuntimePreflight;
	workerCudaStatus: WorkerCudaStatus;
	workerRuntimeBootstrap: WorkerRuntimeBootstrap;
	runtimeProfile: RuntimeProfileSnapshot;
	diagnosticsLogPath: string | null;
	readyForTranscription: boolean;
	models: ModelRow[];
};
```

Set `operatingSystem: 'Windows'` in `initialState`, which keeps the browser dev-server preview representative when Tauri commands are unavailable.

Add:

```ts
export const openDiagnosticsLog = () => invoke<void>('open_diagnostics_log');
```

- [ ] **Step 2: Create high-level view-state helpers**

Create `settingsView.ts` with a presentation-only interface:

```ts
import type {
	ModelManagerSnapshot,
	ModelRow,
	WorkerRuntimeBootstrapStatus
} from '$lib/stores/modelManager';

export type SettingsTab = 'app' | 'debug';
export type StatusTone = 'positive' | 'warning' | 'progress' | 'neutral' | 'error';

export type AppStatusView = {
	tone: StatusTone;
	title: string;
	detail: string;
	badge: string | null;
	showHotkey: boolean;
	modelId: ModelRow['modelId'] | null;
};

const preparingStatuses: WorkerRuntimeBootstrapStatus[] = [
	'checking',
	'installing_python_runtime',
	'upgrading_pip',
	'installing_cuda_torch',
	'installing_worker_requirements',
	'falling_back_to_cpu',
	'smoke_testing'
];

export const valueOrFallback = (
	value: string | null | undefined,
	fallback = 'Not available'
) => value?.trim() || fallback;

export const modelLabel = (snapshot: ModelManagerSnapshot, modelId: string | null) =>
	snapshot.models.find((model) => model.modelId === modelId)?.label ?? 'Not available';

export const executionDeviceLabel = (snapshot: ModelManagerSnapshot) => {
	const device = snapshot.runtimeProfile.actualExecutionDevice;
	if (device === 'gpu') return 'NVIDIA GPU';
	if (device === 'cpu_fallback') return 'CPU';
	return 'Not detected';
};

export const selectAppStatus = (
	snapshot: ModelManagerSnapshot,
	activatingModelId: ModelRow['modelId'] | null
): AppStatusView => {
	const failedModel = snapshot.models.find((model) => model.errorMessage !== null);
	if (snapshot.workerRuntimeBootstrap.status === 'error' || failedModel) {
		return {
			tone: 'error',
			title: 'Needs attention',
			detail: failedModel
				? `${failedModel.label} could not be prepared.`
				: 'The transcription runtime could not be prepared.',
			badge: null,
			showHotkey: false,
			modelId: failedModel?.modelId ?? null
		};
	}

	const downloading = snapshot.models.find((model) => model.status === 'Downloading');
	if (downloading) {
		return {
			tone: 'progress',
			title: `Downloading ${downloading.label}`,
			detail: 'Installing the model on this device.',
			badge: null,
			showHotkey: false,
			modelId: downloading.modelId
		};
	}

	if (activatingModelId) {
		return {
			tone: 'progress',
			title: `Preparing ${modelLabel(snapshot, activatingModelId)}`,
			detail: 'This model will become active when it is ready.',
			badge: null,
			showHotkey: false,
			modelId: activatingModelId
		};
	}

	if (preparingStatuses.includes(snapshot.workerRuntimeBootstrap.status)) {
		return {
			tone: 'progress',
			title: 'Preparing transcription',
			detail: 'OK is getting the local runtime ready.',
			badge: null,
			showHotkey: false,
			modelId: snapshot.loadedModelId
		};
	}

	if (snapshot.readyForTranscription) {
		const cpu = snapshot.runtimeProfile.actualExecutionDevice === 'cpu_fallback';
		return cpu
			? {
					tone: 'warning',
					title: 'Ready — CPU mode',
					detail: 'Dictation may be slower. NVIDIA users can set up CUDA for faster transcription.',
					badge: 'CPU',
					showHotkey: true,
					modelId: snapshot.loadedModelId
				}
			: {
					tone: 'positive',
					title: 'Ready to dictate',
					detail: 'NVIDIA GPU acceleration is active.',
					badge: 'GPU',
					showHotkey: true,
					modelId: snapshot.loadedModelId
				};
	}

	return {
		tone: 'neutral',
		title: 'Choose a model to get started',
		detail: 'Download a model below, then activate it for dictation.',
		badge: null,
		showHotkey: false,
		modelId: null
	};
};
```

- [ ] **Step 3: Type-check the mapping module**

Run:

```bash
npm run check
```

Expected: the new store field and helper module pass TypeScript checking. Existing `Settings.svelte` still compiles before it is replaced.

### Task 3: Rebuild the settings component around App and Debug

**Files:**
- Modify: `src/lib/components/Settings.svelte`

- [ ] **Step 1: Replace advanced runtime imports with the approved action surface**

The component script should import only:

```ts
import { onMount } from 'svelte';
import {
	downloadModel,
	loadModel,
	modelManager,
	openDiagnosticsLog,
	type ModelId,
	type ModelRow
} from '$lib/stores/modelManager';
import {
	executionDeviceLabel,
	modelLabel,
	selectAppStatus,
	valueOrFallback,
	type SettingsTab
} from './settingsView';
```

Remove imports and controls for cache clearing, unloading, model selection, runtime-profile selection, runtime tests, re-detection, and reset.

- [ ] **Step 2: Add local tab, activation, support-error, and progress state**

Use Svelte 5 state and one timer:

```ts
let activeTab = $state<SettingsTab>('app');
let activatingModelId = $state<ModelId | null>(null);
let actionError = $state<string | null>(null);
let downloadProgress = $state<Partial<Record<ModelId, number>>>({});

const appStatus = $derived(selectAppStatus($modelManager, activatingModelId));

onMount(() => {
	const timer = window.setInterval(() => {
		const next = { ...downloadProgress };
		for (const model of $modelManager.models) {
			if (model.status === 'Downloading') {
				const current = next[model.modelId] ?? 0;
				next[model.modelId] = Math.min(90, current + Math.max(1, Math.ceil((90 - current) / 12)));
			} else if (model.status === 'Downloaded' && next[model.modelId] !== undefined) {
				next[model.modelId] = 100;
			} else if (model.status === 'Error') {
				delete next[model.modelId];
			}
		}
		downloadProgress = next;
	}, 700);

	return () => window.clearInterval(timer);
});

const beginDownload = async (modelId: ModelId) => {
	actionError = null;
	downloadProgress = { ...downloadProgress, [modelId]: 0 };
	try {
		await downloadModel(modelId);
	} catch (error) {
		const next = { ...downloadProgress };
		delete next[modelId];
		downloadProgress = next;
		actionError = String(error);
	}
};

const activate = async (modelId: ModelId) => {
	actionError = null;
	activatingModelId = modelId;
	try {
		await loadModel(modelId);
	} catch (error) {
		actionError = String(error);
	} finally {
		activatingModelId = null;
	}
};

const openLog = async () => {
	actionError = null;
	try {
		await openDiagnosticsLog();
	} catch (error) {
		actionError = String(error);
	}
};
```

- [ ] **Step 3: Build the compact native in-app header**

Use a semantic header without a fake close control:

```svelte
<header class="app-header">
	<div class="brand"><span aria-hidden="true">O</span><strong>OK</strong></div>
	<nav class="tabs" aria-label="Settings sections">
		<button class:active={activeTab === 'app'} aria-pressed={activeTab === 'app'} onclick={() => (activeTab = 'app')}>App</button>
		<button class:active={activeTab === 'debug'} aria-pressed={activeTab === 'debug'} onclick={() => (activeTab = 'debug')}>Debug</button>
	</nav>
</header>
```

- [ ] **Step 4: Build the App status card and model rows**

Render `appStatus` first. Include the fixed shortcut only when `appStatus.showHotkey` is true. CPU mode gets a placeholder CUDA link whose click is prevented:

```svelte
<a href="#cuda-setup" onclick={(event) => event.preventDefault()}>Read the CUDA setup guide →</a>
```

For each model, map actions exactly:

```svelte
{#if model.status === 'NotDownloaded'}
	<button class="model-action" onclick={() => beginDownload(model.modelId)}>Download</button>
{:else if model.status === 'Downloading'}
	<span class="model-progress">{downloadProgress[model.modelId] ?? 0}%</span>
{:else if model.status === 'Downloaded'}
	<button class="model-action" disabled={activatingModelId !== null} onclick={() => activate(model.modelId)}>Activate</button>
{:else if model.status === 'Loading' || activatingModelId === model.modelId}
	<span class="model-state">Preparing…</span>
{:else if model.status === 'Loaded'}
	<span class="active-badge">Active</span>
{:else}
	<button class="model-action" onclick={() => beginDownload(model.modelId)}>Download</button>
{/if}
```

Use human copy below each label: `Active · Using NVIDIA GPU`, `Ready to activate`, `Downloading · N%`, `Not installed`, or `Needs attention`. Do not render raw backend status names or technical error strings on App.

End App with a non-navigating placeholder:

```svelte
<a class="help-link" href="#setup" onclick={(event) => event.preventDefault()}>Need help setting up OK?</a>
```

- [ ] **Step 5: Build the read-only Debug tab**

Render a definition list with four rows:

```svelte
<dl class="diagnostics-list">
	<div><dt>System</dt><dd>{$modelManager.operatingSystem} · {valueOrFallback($modelManager.runtimePreflight.gpuName, 'GPU not detected')} · Driver {valueOrFallback($modelManager.runtimePreflight.driverVersion)}</dd></div>
	<div><dt>Acceleration</dt><dd>{$modelManager.workerCudaStatus.cudaAvailable ? 'CUDA available' : valueOrFallback($modelManager.workerCudaStatus.reason, 'CUDA not available')} · CUDA {valueOrFallback($modelManager.workerCudaStatus.cudaVersion)} · PyTorch {valueOrFallback($modelManager.workerCudaStatus.torchVersion)} · {executionDeviceLabel($modelManager)}</dd></div>
	<div><dt>Runtime</dt><dd>{$modelManager.readyForTranscription ? 'Ready' : 'Not ready'} · {$modelManager.workerRuntimeBootstrap.status} · {valueOrFallback($modelManager.runtimeProfile.lastError ?? $modelManager.runtimeProfile.lastTestResult?.error)}</dd></div>
	<div><dt>Model</dt><dd>Selected: {modelLabel($modelManager, $modelManager.selectedModelId)} · Active: {modelLabel($modelManager, $modelManager.loadedModelId)}</dd></div>
</dl>
```

Keep raw bootstrap status in Debug only. Add `Open log` and a disabled `Copy issue report` button with `title="Coming later"`. Show `actionError` as a compact inline message in Debug when Open log fails.

- [ ] **Step 6: Implement the Concept 05A visual system in scoped CSS**

Use these tokens and layout rules as the CSS baseline:

```css
.settings-shell {
	min-height: 100vh;
	background: #111212;
	color: #f4f4f0;
}

.settings-window {
	width: 100%;
	min-height: 100vh;
	background: #202120;
}

.app-header {
	height: 3.25rem;
	display: grid;
	grid-template-columns: 1fr auto 1fr;
	align-items: center;
	padding: 0 1rem;
	border-bottom: 1px solid #484a48;
}

.content {
	padding: 1.25rem;
}

.status-card,
.models-list,
.diagnostics-list {
	border: 1px solid #50534f;
	border-radius: 0.75rem;
}

button:focus-visible,
a:focus-visible {
	outline: 2px solid #d5f18d;
	outline-offset: 2px;
}

@media (max-width: 540px), (max-height: 590px) {
	.content { padding: 1rem; }
	.app-header { height: 3rem; }
}
```

Add the semantic state treatments explicitly:

```css
.status-card.positive { border-color: #727772; background: #292b29; }
.status-card.positive .status-icon { background: #c7f164; color: #162000; }
.status-card.warning { border-color: #947a35; background: #372d13; color: #fff2c9; }
.status-card.warning .status-icon { background: #ffe28c; color: #3a2a00; }
.status-card.progress { border-color: #5c7180; background: #202c33; }
.status-card.progress .status-icon { background: #9ccbe5; color: #10222c; }
.status-card.error { border-color: #8a4d4d; background: #351f1f; color: #ffd9d9; }
.status-card.error .status-icon { background: #ffaaaa; color: #351010; }
.status-card.neutral { border-color: #5d605d; background: #292b29; }

.content {
	overflow-y: auto;
	max-height: calc(100vh - 3.25rem);
}

button:disabled {
	opacity: 0.52;
	cursor: not-allowed;
}
```

Use 12–16px type throughout. Give model rows a `#444643` divider, actions a visible `#777a74` border, the Active badge a `#3b4430` background with `#d8f59a` text, and progress bars a `#9ccbe5` fill over a `#303b41` track. These fixed tokens preserve the chosen Concept 05A contrast and make the visual implementation deterministic.

- [ ] **Step 7: Type-check and build the redesigned frontend**

Run:

```bash
npm run check
npm run build
```

Expected: zero Svelte/TypeScript errors and a successful static production build.

### Task 4: Set compact native window dimensions and global dark baseline

**Files:**
- Modify: `src-tauri/tauri.conf.json`
- Modify: `src/app.css`

- [ ] **Step 1: Update only the main Tauri window dimensions**

Change the `main` window entry to:

```json
{
  "label": "main",
  "title": "Settings",
  "url": "/",
  "width": 600,
  "height": 640,
  "minWidth": 520,
  "minHeight": 560,
  "resizable": true,
  "fullscreen": false,
  "visible": false
}
```

Do not add `decorations: false`; native title-bar behavior is an approved requirement. Do not modify the overlay window.

- [ ] **Step 2: Replace the light global baseline**

Update `src/app.css` root/body colors while retaining the existing resets:

```css
:root {
	font-family: 'Segoe UI Variable Text', 'Segoe UI', system-ui, sans-serif;
	color: #f4f4f0;
	background: #111212;
	color-scheme: dark;
	font-synthesis: none;
	text-rendering: optimizeLegibility;
	-webkit-font-smoothing: antialiased;
	-moz-osx-font-smoothing: grayscale;
}

html,
body {
	margin: 0;
	min-height: 100%;
	background: #111212;
}
```

Keep the global `box-sizing` and form-control font inheritance rules.

- [ ] **Step 3: Validate configuration and production build**

Run:

```bash
npm run check
npm run build
cargo check --manifest-path src-tauri/Cargo.toml
```

Expected: all commands succeed; Tauri accepts `minWidth` and `minHeight`.

### Task 5: Verify behavior and visual states through the app dev server

**Files:**
- Verify: `src/lib/components/Settings.svelte`
- Verify: `src/lib/components/settingsView.ts`
- Verify: `src-tauri/tauri.conf.json`

- [ ] **Step 1: Start the existing app development workflow**

Run the project’s existing Tauri development command from Windows so the frontend receives real snapshot events:

```powershell
npm run tauri -- dev
```

Expected: the tray app starts, the main window opens from the tray, and the native Windows title bar surrounds the compact dark UI.

- [ ] **Step 2: Verify App state mapping**

Exercise or inspect these states without changing backend runtime/profile controls:

- No active model: neutral setup card; Download is available.
- Downloading: simulated percentage advances toward 90% and never reports completion early.
- Download complete: percentage reaches 100%, then the row offers Activate.
- Download failure: progress disappears and Needs attention appears.
- Activating: the status card and row immediately show Preparing.
- GPU ready: quiet green Ready to dictate state and fixed hotkey reminder.
- CPU ready: yellow informational state, no red error treatment, and placeholder CUDA link does not navigate.

Expected: no raw enum, profile ID, package version, path, cache control, unload action, or runtime control appears on App.

- [ ] **Step 3: Verify Debug and support actions**

Confirm Debug shows System, Acceleration, Runtime, and Model values with `Not detected`/`Not available` fallbacks. Click Open log and verify Explorer selects `ok-debug.log`. Confirm Copy issue report is disabled and does not alter the clipboard.

- [ ] **Step 4: Verify both approved window sizes visually**

Capture the live App and Debug views at `600 × 640`, then resize to `520 × 560`. Check that the default ready view does not scroll, the minimum view remains usable, text does not clip, model actions remain visible, and Debug scrolls within the content area when required.

- [ ] **Step 5: Run the final verification suite**

Run:

```bash
npm run check
npm run build
cargo test --manifest-path src-tauri/Cargo.toml
git diff --check
git status --short
```

Expected: checks and tests pass, `git diff --check` reports no whitespace errors, and status shows only the approved files plus the user’s pre-existing changes. Do not stage or commit.
