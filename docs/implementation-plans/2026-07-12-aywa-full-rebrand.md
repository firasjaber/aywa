# aywa Full-Rebrand Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Ship the speech tray application as aywa, with the supplied icon suite and the `com.aywa.speech` Tauri identifier.

**Architecture:** Tauri bundle configuration references the supplied native desktop assets directly. The runtime tray reads the dedicated PNG asset embedded in the Rust binary. SvelteKit copies the supplied favicon assets into its static output and declares them in the document head. All product-owned identifiers move from `ok` to `aywa`, intentionally creating a new application-data namespace.

**Tech Stack:** Tauri 2, Rust, SvelteKit 2, TypeScript, npm.

---

## File structure

- `src-tauri/tauri.conf.json`: product name, identifier, and bundle icon declarations.
- `src-tauri/src/main.rs`: aywa tray icon and crate imports.
- `src-tauri/Cargo.toml`, `src-tauri/Cargo.lock`, and Rust sources: Rust package identity, product-owned constants, fixtures, diagnostics, and test references.
- `package.json` and `package-lock.json`: npm package identity and aywa-prefixed development environment variables.
- `static/favicon/*` and `src/app.html`: published web icon assets and metadata.
- `src/**/*.svelte`, `src/**/*.ts`, `docs/*.md`, `README.md`: user-facing product naming and documented runtime variables.

### Task 1: Reconfigure desktop and web icons

**Files:**
- Modify: `src-tauri/tauri.conf.json`
- Modify: `src-tauri/src/main.rs`
- Create: `static/favicon/favicon-16x16.png`
- Create: `static/favicon/favicon-32x32.png`
- Create: `static/favicon/favicon.ico`
- Create: `static/favicon/apple-touch-icon.png`
- Create: `static/favicon/android-chrome-192x192.png`
- Create: `static/favicon/android-chrome-512x512.png`
- Create: `static/favicon/site.webmanifest`
- Modify: `src/app.html`

- [x] **Step 1: Point Tauri bundling at supplied platform assets**

Set `productName` to `aywa`, set `identifier` to `com.aywa.speech`, and set `bundle.icon` to the supplied Windows ICO, macOS ICNS, Linux hicolor PNGs, and scalable SVG paths under `../static/logo/desktop`.

- [x] **Step 2: Use the dedicated runtime tray asset**

Replace `default_window_icon()` in `build_tray` with:

```rust
let icon = tauri::image::Image::from_bytes(include_bytes!(
    "../../static/logo/tray/aywa-tray-color-32.png"
))?;
```

- [x] **Step 3: Publish and declare favicon assets**

Copy the seven provided files from `static/logo/favicon` into `static/favicon`, then replace the empty data favicon in `src/app.html` with PNG, ICO, Apple-touch, manifest, and theme-color declarations.

- [x] **Step 4: Validate the asset wiring**

Run: `npm run check`

Expected: Svelte checks complete successfully.

### Task 2: Rename package, crate, runtime, and user-facing identity

**Files:**
- Modify: `package.json`
- Modify: `package-lock.json`
- Modify: `src-tauri/Cargo.toml`
- Modify: `src-tauri/Cargo.lock`
- Modify: `src-tauri/src/main.rs`
- Modify: `src-tauri/src/bin/model_manager_probe.rs`
- Modify: product-owned source, test fixtures, and visible Svelte files found by an identity search

- [x] **Step 1: Rename formal package identities**

Set npm `name` to `aywa`. Set Cargo package name and `default-run` to `aywa`, library name to `aywa_lib`, and update imports from `ok_lib` to `aywa_lib`. Regenerate the lockfiles with the package managers rather than manually editing lockfile checksums.

- [x] **Step 2: Rename product-specific runtime markers and variables**

Change `OK_PYTHON_RUNTIME` and `OK_PYTHON_EXE` to `AYWA_PYTHON_RUNTIME` and `AYWA_PYTHON_EXE`; change `ok-python-runtime.json` and `ok-model-manager-*` test fixture prefixes to aywa equivalents; update corresponding diagnostics and documentation.

- [x] **Step 3: Rename UI and release-facing wording**

Replace `OK` product labels, page titles, links, window titles, help text, sample product paths, and documented `com.ok.speech` data locations with aywa equivalents. Leave Rust's `Ok(...)` expressions unchanged.

- [x] **Step 4: Verify Rust compilation and tests**

Run: `cargo test --manifest-path src-tauri/Cargo.toml`

Expected: all existing Rust tests pass under the renamed crate.

### Task 3: Validate the complete rebrand

**Files:**
- Modify: `docs/superpowers/plans/2026-07-12-aywa-full-rebrand.md` (mark completed steps)

- [x] **Step 1: Run frontend validation**

Run: `npm run check`

Expected: no Svelte or TypeScript errors.

- [ ] **Step 2: Validate Tauri configuration and icons**

Run: `npm.cmd run tauri -- build --debug --bundles msi`

Expected: Tauri completes the debug build and accepts all configured icon paths.

- [x] **Step 3: Search for stale product identity**

Run: `rg -n -i --glob '!node_modules/**' --glob '!target/**' --glob '!docs/superpowers/**' --glob '!static/logo/**' '(com\\.ok\\.speech|\\bOK_PYTHON|ok-python-runtime|\\bOK Settings\\b|\\bOK Overlay\\b|>OK<)' .`

Expected: no results. Manually confirm intentional Rust `Ok` expressions are the only remaining unrelated text matches.

- [ ] **Step 4: Commit the rebrand**

Run: `git add package.json package-lock.json src src-tauri static docs README.md && git commit -m "feat: rebrand app as aywa"`

Expected: one commit containing the rebrand and provided logo assets.
