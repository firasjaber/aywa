# Taller default window design

## Goal

Open Aywa's Settings window tall enough that the normal App tab does not initially need vertical scrolling after the History-saving control was added.

## Design

Change only the main window's configured initial height from `420` to `560` pixels in `src-tauri/tauri.conf.json`.

Keep the existing `600` pixel width, `520` pixel minimum width, `460` pixel minimum height, resizable behavior, hidden startup behavior, and overlay window dimensions unchanged. Users can still resize the Settings window smaller after it opens.

The `560` pixel value is a trial default: it is large enough to accommodate the normal App-tab content while remaining materially smaller than a large desktop window. No CSS or content layout changes are included.

## Verification

Validate the Tauri configuration, run the existing frontend checks and production build, and open Aywa once to confirm the App tab initially has no vertical scrollbar under normal ready-state content. The user will report whether the trial height should be adjusted further.
