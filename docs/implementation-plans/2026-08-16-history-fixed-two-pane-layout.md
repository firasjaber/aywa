# History Fixed Two-Pane Layout Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Keep History’s sidebar and transcript side by side in every window size while making the controls more compact.

**Architecture:** Change only scoped CSS in the History Svelte component. Use a fixed 185px grid column, preserve horizontal layout under narrow widths, and retain all existing data and interaction code.

**Tech Stack:** Svelte 5, CSS, Svelte Check.

---

### Task 1: Fix History layout density and responsiveness

**Files:**

- Modify: `src/lib/components/History.svelte:170-177`

- [ ] **Step 1: Keep the grid horizontal at every width**

Replace the proportional column definition and narrow viewport override with:

```css
.history-layout {
    grid-template-columns: 11.5625rem minmax(18rem, 1fr);
    min-width: 29.5625rem;
}
.history { overflow-x: auto; }
```

Delete the `@media (max-width: 38rem)` block that sets `grid-template-columns: 1fr`.

- [ ] **Step 2: Apply compact visual spacing**

Set `.history-row` padding to `.625rem`, `.history-detail` padding to `.75rem`, `.transcript` padding to `.5rem` and top margin to `.75rem`, and action-button font size/padding to `11px` and `.35rem .55rem`. Set `.clear` margin to `.5rem`.

- [ ] **Step 3: Validate the static UI**

Run:

```powershell
npm run check
```

Expected: `svelte-check found 0 errors and 0 warnings`.

- [ ] **Step 4: Inspect default and narrow windows**

Run `npm run tauri -- dev`. Verify the default settings window has a 185px History list beside the transcript. Narrow the window and confirm the panel scrolls horizontally rather than stacking vertically.

- [ ] **Step 5: Commit**

```powershell
git add src/lib/components/History.svelte
git commit -m "fix: keep history panes side by side"
```

## Plan self-review

- **Spec coverage:** The sole task fixes permanent side-by-side layout, fixed sidebar width, horizontal overflow, and compact spacing without modifying data behavior.
- **Placeholder scan:** CSS declarations, file path, commands, and expected outcome are concrete.
- **Type consistency:** No TypeScript or Rust contracts change.
