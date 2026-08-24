# Dedicated Settings Tab Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Move the shortcut recorder and History-saving preference from App into an always-available Settings tab, then restore the main window's previous configured height.

**Architecture:** Extend the pure tab view model with a fixed four-tab definition and render navigation from it so order, labels, and panel IDs share one source. Reuse the existing preference components in a new Settings panel, convert the shortcut component into a standalone titled card, and remove the obsolete model-dependent `showHotkey` view state.

**Tech Stack:** Svelte 5, TypeScript, Vitest, Tauri 2 JSON configuration

---

### Task 1: Define and test the four-tab navigation model

**Files:**
- Modify: `src/lib/components/settingsView.ts`
- Create: `src/lib/components/settingsView.test.ts`

- [ ] **Step 1: Add a failing navigation-order test**

Create `settingsView.test.ts`:

```ts
import { describe, expect, it } from 'vitest';
import { SETTINGS_TABS } from './settingsView';

describe('settings navigation', () => {
	it('places Settings second and exposes matching panel ids', () => {
		expect(SETTINGS_TABS).toEqual([
			{ id: 'app', label: 'App' },
			{ id: 'settings', label: 'Settings' },
			{ id: 'history', label: 'History' },
			{ id: 'debug', label: 'Debug' }
		]);
		expect(SETTINGS_TABS.map((tab) => `${tab.id}-panel`)).toEqual([
			'app-panel', 'settings-panel', 'history-panel', 'debug-panel'
		]);
	});
});
```

- [ ] **Step 2: Run the focused test and verify failure**

Run `npm test -- src/lib/components/settingsView.test.ts`.

Expected: FAIL because `SETTINGS_TABS` does not exist.

- [ ] **Step 3: Implement the tab model and remove shortcut visibility state**

Add:

```ts
export type SettingsTab = 'app' | 'settings' | 'history' | 'debug';

export const SETTINGS_TABS: ReadonlyArray<{ id: SettingsTab; label: string }> = [
	{ id: 'app', label: 'App' },
	{ id: 'settings', label: 'Settings' },
	{ id: 'history', label: 'History' },
	{ id: 'debug', label: 'Debug' }
];
```

Remove `showHotkey` from `AppStatusView` and every `selectAppStatus` result. Do not change status titles, details, tones, or badges.

- [ ] **Step 4: Run the test**

Run `npm test -- src/lib/components/settingsView.test.ts`.

Expected: PASS.

### Task 2: Add the Settings panel and move both cards

**Files:**
- Modify: `src/lib/components/Settings.svelte`
- Modify: `src/lib/components/ShortcutPreference.svelte`
- Modify: `src/lib/components/HistoryPreference.svelte`

- [ ] **Step 1: Render navigation from the shared definition**

Import `SETTINGS_TABS` and replace the repeated buttons with:

```svelte
<div class="tabs" aria-label="Settings sections" role="tablist">
	{#each SETTINGS_TABS as tab}
		<button
			type="button"
			class:active={activeTab === tab.id}
			role="tab"
			aria-selected={activeTab === tab.id}
			aria-controls={`${tab.id}-panel`}
			onclick={() => (activeTab = tab.id)}>{tab.label}</button
		>
	{/each}
</div>
```

- [ ] **Step 2: Move the preferences**

Remove `<ShortcutPreference />` and `<HistoryPreference />` from App. Insert before History:

```svelte
{:else if activeTab === 'settings'}
	<div class="content preferences-content" id="settings-panel" role="tabpanel">
		<ShortcutPreference />
		<HistoryPreference />
	</div>
{:else if activeTab === 'history'}
```

Add:

```css
.preferences-content {
	display: grid;
	align-content: start;
	gap: 1rem;
}
```

- [ ] **Step 3: Make the shortcut recorder a titled card**

Wrap its current UI in:

```svelte
<section class="shortcut-preference" aria-labelledby="shortcut-preference-heading">
	<div class="preference-heading">
		<h2 id="shortcut-preference-heading">Dictation shortcut</h2>
		<p>Choose the keys you hold while speaking.</p>
	</div>
	<!-- existing shortcut UI, saving state, and error -->
</section>
```

Give `.shortcut-preference` the same grid gap, border, radius, padding, and background as `HistoryPreference`. Set its inner `.hotkey` margin to zero. Change `.history-preference` from `margin-top: 1.1rem` to `margin-top: 0` because the panel owns spacing.

- [ ] **Step 4: Verify frontend behavior**

Run:

```powershell
npm test
npm run check
npm run build
```

Expected: all frontend tests, Svelte checks, and the production build pass.

### Task 3: Restore height and update instructions

**Files:**
- Modify: `src-tauri/tauri.conf.json`
- Modify: `docs/USER_GUIDE.md`
- Modify: `docs/TROUBLESHOOTING.md`

- [ ] **Step 1: Revert only the main default height**

Use:

```json
"width": 600,
"height": 420,
"minWidth": 520,
"minHeight": 460
```

Do not change the overlay or other window properties.

- [ ] **Step 2: Point preferences to Settings**

Change Change-shortcut, Reset-to-default, active-shortcut display, and Save-to-History instructions from App to Settings. Retain App references for readiness and model activation.

- [ ] **Step 3: Verify references**

Run:

```powershell
rg -n "App tab|Settings tab|Change shortcut|Reset to default|Save transcriptions" docs/USER_GUIDE.md docs/TROUBLESHOOTING.md
rg -n '"height"|"minHeight"' src-tauri/tauri.conf.json
```

Expected: preferences point to Settings; main height is 420/minHeight 460; overlay height remains 44.

### Task 4: Full regression and visual verification

**Files:**
- Verify all changed files

- [ ] **Step 1: Run complete automated checks**

```powershell
npm test
npm run check
npm run build
cargo test --manifest-path src-tauri/Cargo.toml
git diff --check
```

Expected: 4 frontend test files pass, Svelte has no diagnostics, build and Rust tests pass, and no whitespace errors appear.

- [ ] **Step 2: Browser-check at 600×460**

Verify the four-tab order, compact App page, always-accessible Settings page, two separate preference cards, scrolling/footer behavior, and unchanged History/Debug panels.

- [ ] **Step 3: Commit**

```powershell
git add src/lib/components/settingsView.ts src/lib/components/settingsView.test.ts src/lib/components/Settings.svelte src/lib/components/ShortcutPreference.svelte src/lib/components/HistoryPreference.svelte src-tauri/tauri.conf.json docs/USER_GUIDE.md docs/TROUBLESHOOTING.md
git commit -m "ui: move preferences to settings tab"
```
