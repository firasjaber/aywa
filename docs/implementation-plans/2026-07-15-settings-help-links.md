# Settings Help Links Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make the two Settings help links open their corresponding hosted Aywa documentation in the default browser.

**Architecture:** Install Tauri's opener plugin and expose its narrowly scoped `openUrl` JavaScript API to Settings. Keep URL selection and error handling local to `Settings.svelte`, so the existing `actionError` UI handles launch failures without new application state or commands.

**Tech Stack:** Svelte 5 and TypeScript; Tauri 2; `@tauri-apps/plugin-opener` and `tauri-plugin-opener`.

---

## File structure

- `package.json`: adds the Tauri opener JavaScript dependency.
- `package-lock.json`: records the resolved JavaScript dependency.
- `src-tauri/Cargo.toml` and `src-tauri/Cargo.lock`: add the compatible Rust opener plugin.
- `src-tauri/src/main.rs`: registers the plugin during Tauri builder setup.
- `src-tauri/capabilities/default.json`: permits only the two intended GitHub documentation URLs.
- `src/lib/components/Settings.svelte`: invokes the opener and replaces the two inert fragment handlers.

### Task 1: Install and authorize the Tauri opener plugin

**Files:**
- Modify: `package.json`
- Modify: `package-lock.json`
- Modify: `src-tauri/Cargo.toml`
- Modify: `src-tauri/Cargo.lock`
- Modify: `src-tauri/src/main.rs:204-244`
- Modify: `src-tauri/capabilities/default.json:1-12`

- [ ] **Step 1: Add the official opener plugin dependencies**

Run:

```powershell
npm run tauri add opener
```

Expected: `package.json` gains `@tauri-apps/plugin-opener`, and `src-tauri/Cargo.toml` gains `tauri-plugin-opener`, with the matching lockfiles updated.

- [ ] **Step 2: Register the Rust plugin**

Update the builder chain in `src-tauri/src/main.rs`:

```rust
fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(AppStateManager::default())
```

- [ ] **Step 3: Limit frontend authorization to the two documentation pages**

Replace the capability permissions array in `src-tauri/capabilities/default.json` with:

```json
"permissions": [
  "core:default",
  {
    "identifier": "opener:allow-open-url",
    "allow": [
      { "url": "https://github.com/firasjaber/aywa/blob/main/docs/TROUBLESHOOTING.md#cuda-is-not-available" },
      { "url": "https://github.com/firasjaber/aywa/blob/main/docs/USER_GUIDE.md" }
    ]
  }
]
```

- [ ] **Step 4: Verify Rust configuration compiles**

Run:

```powershell
cargo check --manifest-path src-tauri/Cargo.toml
```

Expected: `Finished` with no capability-schema or missing-plugin errors.

- [ ] **Step 5: Commit the plugin setup**

```powershell
git add package.json package-lock.json src-tauri/Cargo.toml src-tauri/Cargo.lock src-tauri/src/main.rs src-tauri/capabilities/default.json
git commit -m "feat: add external documentation opener"
```

### Task 2: Wire Settings help links to the hosted documentation

**Files:**
- Modify: `src/lib/components/Settings.svelte:1-125`
- Modify: `src/lib/components/Settings.svelte:221-226`
- Modify: `src/lib/components/Settings.svelte:307-310`

- [ ] **Step 1: Import the opener and define the two fixed URLs**

Add this import after the Svelte import:

```ts
import { openUrl } from '@tauri-apps/plugin-opener';
```

Add these constants after the imports:

```ts
const cudaTroubleshootingUrl =
    'https://github.com/firasjaber/aywa/blob/main/docs/TROUBLESHOOTING.md#cuda-is-not-available';
const userGuideUrl = 'https://github.com/firasjaber/aywa/blob/main/docs/USER_GUIDE.md';
```

- [ ] **Step 2: Add the shared browser-launch action**

After `openLog`, add:

```ts
const openHelp = async (url: string) => {
    actionError = null;
    try {
        await openUrl(url);
    } catch (error) {
        actionError = errorText(error);
    }
};
```

- [ ] **Step 3: Replace the CUDA fragment handler**

Replace the warning link with:

```svelte
<a href={cudaTroubleshootingUrl} onclick={(event) => {
    event.preventDefault();
    openHelp(cudaTroubleshootingUrl);
}}>Read the CUDA setup guide →</a>
```

- [ ] **Step 4: Replace the setup fragment handler**

Replace the setup link with:

```svelte
<a class="help-link" href={userGuideUrl} onclick={(event) => {
    event.preventDefault();
    openHelp(userGuideUrl);
}}>Need help setting up Aywa?</a>
```

- [ ] **Step 5: Verify static checks and production build**

Run:

```powershell
npm run check
npm run build
```

Expected: both commands exit 0, with no Svelte type, accessibility, or Vite build errors.

- [ ] **Step 6: Manually verify both live links**

Run:

```powershell
npm run tauri -- dev
```

Expected: with Settings open, select **Need help setting up Aywa?** and confirm the default browser opens the hosted user guide. Make the app use CPU mode, select **Read the CUDA setup guide →**, and confirm the browser opens the CUDA troubleshooting heading. Confirm the Settings window remains open and no error message appears.

- [ ] **Step 7: Commit the Settings change**

```powershell
git add src/lib/components/Settings.svelte
git commit -m "fix: open settings help documentation"
```

