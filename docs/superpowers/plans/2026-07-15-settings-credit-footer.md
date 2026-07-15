# Settings Credit Footer Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Display linked author and source credit at the bottom of the Settings window on both tabs.

**Architecture:** Keep the footer as one shared element after the conditional App/Debug panel, within the existing scrollable Settings content. Reuse `openHelp` to launch the fixed URLs and surface errors; extend the existing Tauri opener allow-list with those two URLs.

**Tech Stack:** Svelte 5 and TypeScript; Tauri 2; `@tauri-apps/plugin-opener`.

---

## File structure

- `src/lib/components/Settings.svelte`: declares the credit URLs, renders the shared footer, and styles its links.
- `src-tauri/capabilities/default.json`: authorizes the two fixed external URLs for the opener plugin.

### Task 1: Authorize the footer destinations

**Files:**
- Modify: `src-tauri/capabilities/default.json:9-18`

- [ ] **Step 1: Add the website and repository addresses to the opener allow-list**

Append these allow entries after the existing user-guide entry:

```json
{
  "url": "https://firrj.com"
},
{
  "url": "https://github.com/firasjaber/aywa"
}
```

- [ ] **Step 2: Check that the Tauri configuration compiles**

Run:

```powershell
cargo check --manifest-path src-tauri/Cargo.toml
```

Expected: exits with status 0 and reports no capability-schema errors.

- [ ] **Step 3: Commit the authorization change**

```powershell
git add src-tauri/capabilities/default.json
git commit -m "feat: authorize settings credit links"
```

### Task 2: Add the shared Settings footer

**Files:**
- Modify: `src/lib/components/Settings.svelte:18-21`
- Modify: `src/lib/components/Settings.svelte:330-332`
- Modify: `src/lib/components/Settings.svelte:587-590`

- [ ] **Step 1: Declare the fixed footer URLs**

Add alongside the existing help URL constants:

```ts
const authorWebsiteUrl = 'https://firrj.com';
const sourceCodeUrl = 'https://github.com/firasjaber/aywa';
```

- [ ] **Step 2: Render one footer after the tab conditional**

Add this immediately after the `{#if activeTab ...}{/if}` block and before the closing `</section>` for `.settings-window`:

```svelte
<footer class="settings-footer">
  Built by
  <a href={authorWebsiteUrl} onclick={(event) => {
    event.preventDefault();
    openHelp(authorWebsiteUrl);
  }}>Firas Jaber</a>
  <span aria-hidden="true">â€”</span>
  <a href={sourceCodeUrl} onclick={(event) => {
    event.preventDefault();
    openHelp(sourceCodeUrl);
  }}>View source code</a>
</footer>
```

- [ ] **Step 3: Style the footer as a compact, normal-flow element**

Add these rules before the small-screen media query:

```css
.settings-footer {
  padding: 0 1.25rem 1.25rem;
  color: #bcbeb9;
  font-size: 12px;
  text-align: center;
}

.settings-footer a {
  color: #d5f18d;
  font-weight: 700;
  text-decoration: underline;
  text-underline-offset: 0.22rem;
}
```

Update the existing compact-screen `.content` padding to leave the footer horizontal alignment unchanged; no footer positioning rule is needed.

- [ ] **Step 4: Run static checks and the production build**

Run:

```powershell
npm run check
npm run build
```

Expected: both commands exit 0 without Svelte, TypeScript, accessibility, or Vite errors.

- [ ] **Step 5: Manually verify external-link behavior**

Run:

```powershell
npm run tauri -- dev
```

Expected: on both **App** and **Debug**, the footer is visible after the panel content. Selecting **Firas Jaber** opens `https://firrj.com`; selecting **View source code** opens the Aywa GitHub repository. The Settings window remains open and no error is shown.

- [ ] **Step 6: Commit the footer implementation**

```powershell
git add src/lib/components/Settings.svelte
git commit -m "feat: add settings credit footer"
```
