# Settings Credit Footer Design

## Goal

Show a consistent author and source-credit footer in the Settings window on both the App and Debug tabs.

## Scope

- Render one shared footer below the active tab panel, rather than duplicating it within both panels.
- Display the text `Built by Firas Jaber — View source code`.
- Link `Firas Jaber` to `https://firrj.com` and `View source code` to `https://github.com/firasjaber/aywa`.
- Open each link through the existing Tauri external-browser helper.
- Keep the footer in normal document flow so it follows scrollable content and does not obscure controls.

## Non-goals

- Add a new Settings tab or in-app browser.
- Change existing Help, diagnostics, or model-management behavior.

## Behavior

Selecting either footer link prevents in-window navigation and opens the target HTTPS address in the user's default browser. If opening a link fails, the existing Settings action-error surface displays the failure. Switching tabs keeps the same footer visible below the selected panel.

## Verification

- `npm run check` passes.
- In a Tauri development build, the author link opens `https://firrj.com` and the source link opens the Aywa GitHub repository.
- The footer is visible and usable from both App and Debug tabs, without covering content at small window sizes.
