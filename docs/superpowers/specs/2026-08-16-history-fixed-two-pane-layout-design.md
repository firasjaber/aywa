# History fixed two-pane layout design

## Goal

Keep Aywa History’s list and transcript detail side by side at the default window size and all narrower widths.

## Design

The History grid uses a fixed 185px sidebar and a `minmax(0, 1fr)` detail pane. The responsive rule that turns the grid into one column is removed. When a window is narrower than the combined usable width, the History panel remains horizontal and allows horizontal scrolling rather than moving detail below the list.

Reduce visual density without changing content or behavior: history rows use 10px padding; detail uses 12px; transcript text uses 8px; and action buttons use 11px text with compact padding. List previews remain clamped to two lines.

## Scope and verification

Only `src/lib/components/History.svelte` styles change. Storage, History commands, selection, pagination, and deletion behavior remain unchanged. Verify `npm run check`, then inspect History in Aywa’s default-size settings window and at a narrower width to confirm the two panes never stack.
