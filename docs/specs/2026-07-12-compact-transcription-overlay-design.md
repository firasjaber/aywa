# Compact Transcription Overlay Design

## Purpose

Make the transcription overlay quiet, compact, and visually consistent with the dark OK settings window. It must avoid showing local recording paths and make active transcription immediately recognizable without expanding the UI.

## Chosen approach

Use focused visual polish in the existing overlay and settings components. Retain the overlay window's existing native transparency; only the compact overlay card is rendered. This avoids backend or window-management changes while removing the visible black canvas problem.

## Overlay

The overlay remains a transparent, frameless, always-on-top Tauri window. Its card is a small dark neutral surface related to the settings window palette, with a faint neutral border, a modest rounded corner radius, and a restrained shadow. It is positioned at the bottom-right as today.

The card is reduced from its current presentation: narrower minimum width, smaller padding, tighter icon-to-text gap, and smaller text. Message text stays short and human-readable.

State behavior:

- Listening displays the recording indicator and listening copy. The amber recording indicator has a repeating pulse; the card itself does not animate.
- Processing displays short transcription copy such as `Transcribing...` with no file-system path, recording filename, or other technical detail.
- Completed and error retain their success and error color identities but remain compact and static.
- `prefers-reduced-motion` disables the recording pulse.

The app already configures the overlay window with `transparent: true`; the implementation must preserve that setting. If a platform cannot render a transparent webview, the card's themed surface remains a graceful fallback rather than a solid black full-window background.

## Settings visual states

In the App status card:

- Progress/loading uses an animated CSS spinner in place of the static ellipsis character. It remains decorative while the existing title/detail text communicates status to assistive technology.
- The neutral setup state (`Choose a model to get started`) uses a download-arrow icon rather than the letter `O`. The icon stays inside the current circular neutral status treatment.

No model-management data flow, backend status mapping, or action behavior changes are required.

## Implementation boundaries

- Modify `src/routes/overlay/+page.svelte` for compact themed overlay presentation, pulse behavior, and reduced-motion handling.
- Modify `src/lib/components/Settings.svelte` for inline SVG/CSS spinner and download-arrow presentation.
- Do not change Tauri commands, model-manager contracts, hotkeys, recording behavior, or overlay positioning logic except where the configured window dimensions need to shrink to fit the compact card.
- If the current processing message is created by recording/transcription state code rather than the overlay component, replace only the path-bearing message with the concise transcription label at its source.

## Error handling and accessibility

- Preserve `aria-live="polite"` for overlay messages.
- Mark decorative indicator, spinner, and icon SVGs as hidden from assistive technology.
- Do not rely on motion or color alone to communicate any state.
- Keep the existing error message text visible while applying only visual changes.

## Verification criteria

- The overlay window remains transparent outside the card on supported Windows webviews.
- The card is visibly smaller than the current 320 x 84 presentation and has the same dark design language as Settings.
- Only the recording indicator pulses while listening; it does not pulse in reduced-motion mode.
- A post-recording processing overlay reads `Transcribing...` (or equivalent concise copy) and never exposes the saved recording path.
- Loading/progress in the status card visibly spins instead of rendering a static ellipsis.
- The neutral setup status uses a download-arrow icon rather than `O`.
- Svelte type/check validation passes.
