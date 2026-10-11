# Settings Help Links Design

## Goal

Make the Settings links for CUDA recovery and initial setup useful by opening the matching hosted Aywa documentation in the user's default browser.

## Scope

- The CUDA warning link opens the hosted troubleshooting section: `https://github.com/firasjaber/aywa/blob/main/docs/TROUBLESHOOTING.md#cuda-is-not-available`.
- The setup help link opens the hosted user guide: `https://github.com/firasjaber/aywa/blob/main/docs/USER_GUIDE.md`.
- Both actions use Tauri's supported external-browser capability behind one component-local helper.
- If launching the browser fails, the existing Settings action-error surface shows the error.

## Non-goals

- Change issue-report generation, redaction, clipboard behavior, or support documentation.
- Add in-app documentation rendering or a new settings section.
- Add help links beyond the two currently inert actions.

## Behavior

Selecting either link prevents the fragment navigation and requests the operating system to open the corresponding HTTPS URL. The UI remains on the current Settings tab. A failed launch does not throw to the UI; it updates the existing action error, matching model, log, and report actions.

## Verification

`npm run check` passes. In a Tauri development build, selecting each link opens the expected GitHub documentation page in the default browser; an injected opener failure is surfaced as the existing Settings error message.
