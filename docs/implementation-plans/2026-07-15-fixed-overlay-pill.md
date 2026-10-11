# Fixed Overlay Pill Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make the transcription overlay a compact, centered, fixed-width pill sized for the status icon plus `Transcribing...`.

**Architecture:** Keep the current backend-owned overlay state and transparent Tauri window unchanged. The Svelte component fixes the visible pill dimensions and centers it in a tightly padded shell; the Tauri window is resized to that shell so no excess canvas surrounds it.

**Tech Stack:** Svelte 5, CSS, Tauri v2, TypeScript, npm.

---

## File structure

- `src/lib/components/Overlay.svelte`: renders state-driven overlay content and owns the shell/pill layout styles.
- `src-tauri/tauri.conf.json`: owns the static native overlay window dimensions.

### Task 1: Fix and center the visible pill

**Files:**
- Modify: `src/lib/components/Overlay.svelte:34-61`

- [ ] **Step 1: Update the shell and pill sizing rules**

Replace the existing `.overlay-shell` and `.pill` rules with:

```css
.overlay-shell {
	width: 100%;
	height: 100%;
	display: flex;
	align-items: center;
	justify-content: center;
	padding: 0.25rem;
	box-sizing: border-box;
	pointer-events: none;
}

.pill {
	display: inline-flex;
	box-sizing: border-box;
	align-items: center;
	gap: 0.5rem;
	width: 10rem;
	padding: 0.55rem 0.7rem;
	border: 1px solid rgba(244, 244, 240, 0.16);
	border-radius: 0.6rem;
	background: rgba(32, 33, 32, 0.94);
	box-shadow: 0 8px 20px rgba(0, 0, 0, 0.2);
	backdrop-filter: blur(12px);
	color: #f4f4f0;
}
```

The 10rem width is 160 CSS pixels and leaves room for the current 0.55rem icon, 0.5rem gap, existing horizontal padding, and the uppercase `Transcribing...` label. Do not change the dot, label, state-color, accessibility, or animation rules.

- [ ] **Step 2: Run the Svelte and TypeScript check**

Run: `npm run check`

Expected: exits with code 0 and reports no new errors for `Overlay.svelte`.

- [ ] **Step 3: Commit the component change**

```powershell
git add -- src/lib/components/Overlay.svelte
git commit -m "fix: stabilize overlay pill size"
```

### Task 2: Tighten the native overlay frame

**Files:**
- Modify: `src-tauri/tauri.conf.json:27-42`

- [ ] **Step 1: Match the overlay window to the shell dimensions**

In the `overlay` window object, change only these properties:

```json
"width": 168,
"height": 44,
```

This is the 160px pill plus 4px of outer padding on both horizontal sides. Its 44px height preserves a 4px inset around the roughly 36px pill content while preventing the current 240 by 64 transparent canvas from making the visible card appear off-center.

- [ ] **Step 2: Build the frontend**

Run: `npm run build`

Expected: exits with code 0 and writes the static app bundle to `build`.

- [ ] **Step 3: Manually verify the state sequence in the desktop app**

Run: `npm run tauri -- dev`

Expected:

- `Listening...`, `Transcribing...`, completion, and error all render in the same 160px pill.
- The pill is centered in the native overlay window and has an even, narrow inset on every side.
- `Transcribing...` remains on one line and does not collide with the icon.
- The overlay remains bottom-right aligned when shown, and pulsing/reduced-motion behavior is unchanged.

- [ ] **Step 4: Commit the configuration change**

```powershell
git add -- src-tauri/tauri.conf.json
git commit -m "fix: tighten overlay frame"
```
