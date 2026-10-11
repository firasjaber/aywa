# aywa full-rebrand design

## Goal

Replace the existing `ok` product identity with `aywa` across the desktop application, its bundles, runtime-facing names, and release metadata. The application will use the new Tauri identifier `com.aywa.speech`.

## Scope

### Product and package identity

- Set the displayed product name to `aywa`.
- Set the Tauri application identifier to `com.aywa.speech`.
- Rename JavaScript package metadata and Rust package, library, and default-run names from `ok` to `aywa` where they are product identifiers.
- Update user-visible titles, labels, accessibility text, and help copy to say `aywa`.
- Update release/publish configuration and project documentation that describe the shipped application as `ok`.

### Runtime namespace

- Rename product-specific environment variable names and runtime marker filenames from the `OK`/`ok` prefix to the `AYWA`/`aywa` prefix.
- Update diagnostics and test fixtures that encode the old product path or identifier.
- The new identifier intentionally gives aywa a fresh application-data directory. Existing `com.ok.speech` settings, models, and embedded Python runtime are not migrated.

### Icons and web assets

- Tauri bundling will use the supplied platform-native assets:
  - Windows: `static/logo/desktop/windows/aywa.ico`
  - macOS: `static/logo/desktop/macos/aywa.icns`
  - Linux: the supplied hicolor PNG sizes and scalable SVG under `static/logo/desktop/linux`.
- The runtime tray icon will use the supplied small color tray PNG suitable for the platform rather than the generic bundle icon.
- The Svelte app will expose supplied favicon, Apple touch, Android, and manifest assets from `static/logo/favicon`, referenced from `src/app.html`.

## Implementation boundaries

- Preserve Rust's `Ok(...)` result expressions and unrelated words that happen to contain `ok`.
- Preserve the existing user-provided `static/logo` assets; only reference or copy them as needed by the build and web asset pipelines.
- Do not add compatibility migration behavior for prior installations.

## Verification

- Run the frontend type and Svelte checks.
- Run relevant Rust formatting/check or tests after package/crate renames.
- Build or validate Tauri configuration sufficiently to confirm all configured icon paths exist and are accepted.
- Search product-facing configuration, source, documentation, and release metadata for remaining obsolete `OK`/`ok` identity references, allowing Rust's `Ok` syntax and unrelated text.

