# Aywa icon and title refinement design

## Goal

Use high-resolution supplied icon assets for the desktop application and display the Aywa mark next to the visible application title.

## Scope

- Use `Aywa` for all display-facing application, page, window, and settings-header titles.
- Retain lowercase technical names: npm/Rust package `aywa`, Tauri identifier `com.aywa.speech`, and `AYWA_*` environment variables.
- Configure Tauri with the supplied 1024px app-dark PNG for high-DPI desktop icon rendering, plus the supplied native Windows ICO and macOS ICNS files.
- Add the supplied circular 32px icon before `Aywa` in the settings header, aligned with the existing navigation tabs.

## Boundaries

- Do not change the Tauri identifier, runtime namespace, or user data location.
- Do not generate or modify logo artwork; use only supplied assets under `static/logo`.

## Verification

- Confirm configured Tauri icon paths exist.
- Run `npm.cmd run check` to validate the Svelte header markup and styles.
- Search display-facing titles to confirm they use `Aywa`.

