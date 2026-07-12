# Compact Transcription Overlay Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Give the transcription overlay a compact themed card and make Settings loading/setup states use meaningful icons.

**Architecture:** Retain the transparent native overlay and the current app-state contract. Change only Svelte presentation; the existing Processing state remains the source of `Transcribing...` copy.

**Tech Stack:** Svelte 5, TypeScript, CSS, inline SVG, Tauri 2, Rust tests.

---

### Task 1: Preserve concise processing copy

**Files:**
- Modify: `src-tauri/src/state.rs:165-194`
- Verify: `src-tauri/src/lib.rs:126-155`

- [ ] **Step 1: Add the focused state test**

Add beside the existing processing-message tests:

```rust
#[test]
fn processing_state_uses_concise_transcribing_copy() {
    assert_eq!(
        AppState::Processing.default_message(None),
        Some("Transcribing...".to_string())
    );
}
```

- [ ] **Step 2: Run it before and after the change**

Run: `cargo test processing_state_uses_concise_transcribing_copy`

Expected: PASS.

- [ ] **Step 3: Preserve the path-free transition**

Confirm the recording success path remains:

```rust
transition_to(app, AppState::Processing, None)?;
```

Do not pass `result.path.display()` to `transition_to`; Processing selects the concise default message.

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/state.rs
git commit -m "test: preserve concise transcription overlay copy"
```

### Task 2: Compact the overlay card

**Files:**
- Modify: `src/routes/overlay/+page.svelte:1-105`
- Modify: `src-tauri/tauri.conf.json:22-35` only if visual inspection finds excessive transparent window space

- [ ] **Step 1: Limit pulsing to active recording**

Replace:

```ts
const pulseStates = new Set(['listening', 'processing']);
```

With:

```ts
const pulseStates = new Set(['listening']);
```

- [ ] **Step 2: Apply the compact themed card rules**

Replace the `.overlay-shell`, `.pill`, `.dot`, and `.label` sizing rules with:

```css
.overlay-shell { padding: 0.5rem; }
.pill {
    gap: 0.5rem;
    min-width: 0;
    padding: 0.55rem 0.7rem;
    border: 1px solid rgba(244, 244, 240, 0.16);
    border-radius: 0.6rem;
    background: rgba(32, 33, 32, 0.94);
    box-shadow: 0 8px 20px rgba(0, 0, 0, 0.2);
    backdrop-filter: blur(12px);
    color: #f4f4f0;
}
.dot { width: 0.55rem; height: 0.55rem; }
.label { font-size: 0.75rem; letter-spacing: 0.04em; }
```

Preserve transparent `html` and `body` backgrounds, `pointer-events: none`, and completed/error dot colors.

- [ ] **Step 3: Add motion accessibility**

Reduce the pulse ring expansion from 12px to 8px and add:

```css
@media (prefers-reduced-motion: reduce) {
    .pulse { animation: none; }
}
```

- [ ] **Step 4: Inspect and optionally shrink the native window**

Run: `npm run tauri -- dev`

Keep `transparent: true`, `decorations: false`, and `shadow: false`. If the compact card leaves excessive empty space, change only the overlay window to `width: 240` and `height: 64`; otherwise leave it unchanged.

- [ ] **Step 5: Validate and commit**

Run: `npm run check`

Expected: `svelte-check found 0 errors and 0 warnings`.

```bash
git add src/routes/overlay/+page.svelte src-tauri/tauri.conf.json
git commit -m "feat: compact themed transcription overlay"
```

Stage `src-tauri/tauri.conf.json` only if Step 4 changed it.

### Task 3: Render semantic Settings status icons

**Files:**
- Modify: `src/lib/components/Settings.svelte:74-78,101-107,270-290`

- [ ] **Step 1: Replace text icon selection with icon kinds**

Replace `statusIcon` with:

```ts
const statusIcon = (tone: typeof appStatus.tone) => {
    if (tone === 'positive') return 'check';
    if (tone === 'warning') return 'warning';
    if (tone === 'error') return 'error';
    if (tone === 'progress') return 'spinner';
    return 'download';
};
```

- [ ] **Step 2: Render spinner and download-arrow SVGs**

Replace the text inside the `.status-icon` span with branches that render this progress SVG:

```svelte
<svg class="status-spinner" viewBox="0 0 24 24" aria-hidden="true">
    <circle cx="12" cy="12" r="8" />
</svg>
```

and this neutral setup SVG:

```svelte
<svg viewBox="0 0 24 24" aria-hidden="true">
    <path d="M12 3v11m0 0 4-4m-4 4-4-4M5 19h14" />
</svg>
```

Keep inline check, warning, and error SVG branches for their existing tones; all are decorative because the title/detail communicate the state.

- [ ] **Step 3: Style and animate the spinner**

Add:

```css
.status-icon svg {
    width: 1rem;
    height: 1rem;
    fill: none;
    stroke: currentColor;
    stroke-linecap: round;
    stroke-linejoin: round;
    stroke-width: 2;
}
.status-spinner { stroke-dasharray: 38 12; animation: status-spin 0.9s linear infinite; }
@keyframes status-spin { to { transform: rotate(360deg); } }
@media (prefers-reduced-motion: reduce) { .status-spinner { animation: none; } }
```

- [ ] **Step 4: Validate states and commit**

Run: `npm run check`

Expected: `svelte-check found 0 errors and 0 warnings`.

Run: `npm run dev`

Expected: neutral setup shows a download arrow; progress rotates a spinner; positive, warning, and error remain recognizable and readable.

```bash
git add src/lib/components/Settings.svelte
git commit -m "feat: clarify settings status icons"
```

### Task 4: Final verification

**Files:**
- Verify: `src-tauri/src/state.rs`
- Verify: `src/routes/overlay/+page.svelte`
- Verify: `src/lib/components/Settings.svelte`

- [ ] **Step 1: Run complete frontend validation**

Run: `npm run check`

Expected: `svelte-check found 0 errors and 0 warnings`.

- [ ] **Step 2: Run Rust unit tests**

Run: `cargo test`

Working directory: `src-tauri`

Expected: all tests pass.

- [ ] **Step 3: Exercise the overlay sequence**

Run: `npm run tauri -- dev`

Expected: holding the hotkey produces a compact listening card with only the dot pulsing; releasing shows `Transcribing...` without a path; completed and error cards stay compact and static.

- [ ] **Step 4: Check handoff cleanliness**

Run: `git diff --check`

Expected: no whitespace errors, and no unrelated dirty files staged by this work.

