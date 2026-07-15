# Fixed Overlay Pill Design

## Goal

Keep the transcription status overlay visually stable across its three message states by giving its pill a fixed width sized for the longest active content: the status icon plus `Transcribing...`.

## Design

- Preserve the existing transparent, frameless Tauri overlay window and the current state-to-message behavior.
- Keep the pill's existing dark surface, border, radius, shadow, and status icon treatment.
- Set one fixed pill width that accommodates the icon, the `Transcribing...` label, the existing internal gap, and horizontal padding without wrapping or clipping.
- Center the pill within the overlay shell for every state.
- Retain the shell as the outer overlay background layer, but constrain it to a small, uniform inset around the pill so it appears almost the same size. The inset applies evenly on every side.
- Resize the native overlay window to match the resulting shell dimensions, avoiding surplus transparent canvas while preserving the existing bottom-right placement logic.

## Scope

Change only the overlay component styles and the configured overlay window dimensions if its current dimensions exceed the tight shell. Do not alter app states, status text, icon colors, animation behavior, accessibility, or window positioning behavior.

## Verification

- Run the frontend type and Svelte checks.
- Build the frontend.
- Manually cycle the overlay through listening, transcribing, completion, and error states to confirm the pill remains centered, fixed-size, unclipped, and surrounded by only the intended even inset.
