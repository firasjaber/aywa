# Aywa Icon and Title Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Display Aywa consistently with its high-resolution desktop icon and header mark.

**Architecture:** Tauri will use the supplied 1024px PNG alongside native ICO and ICNS bundle assets. The settings header will render the existing circular PNG from the public static directory. Visible titles use `Aywa`; technical identifiers remain lowercase.

**Tech Stack:** Tauri 2, Svelte 5, TypeScript.

---

### Task 1: Refine desktop and UI branding

**Files:**
- Modify: `src-tauri/tauri.conf.json`
- Modify: `src/routes/+layout.svelte`
- Modify: `src/lib/components/Settings.svelte`

- [x] **Step 1: Use the high-resolution app icon**

Replace the Linux PNG bundle sizes with `../static/logo/png/app-dark/aywa-dark-1024.png`, retaining `../static/logo/desktop/macos/aywa.icns` and `../static/logo/desktop/windows/aywa.ico`.

- [x] **Step 2: Capitalize visible titles**

Set Tauri `productName` to `Aywa`, window titles to `Aywa Settings` and `Aywa Overlay`, and the Svelte document title to `Aywa`.

- [x] **Step 3: Add the header icon**

Replace the current settings `.brand` markup with:

```svelte
<div class="brand">
  <img src="/logo/png/circular/aywa-circle-32.png" alt="" />
  <strong>Aywa</strong>
</div>
```

Style the image at `1.25rem` square so it aligns with the prior brand marker.

- [x] **Step 4: Validate the refinement**

Run: `npm.cmd run check`

Expected: `svelte-check found 0 errors and 0 warnings`.
