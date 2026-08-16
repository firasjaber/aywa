# Transcription history design

## Goal

Add a fully local History tab to Aywa so a person can browse, read, copy, and delete completed transcriptions after the transient overlay has closed. This first release deliberately excludes search, audio retention, editing, export, and cloud synchronization.

## User experience

History is a top-level section alongside the existing App and Debug tabs in the settings window.

The History layout has two panes:

- The left pane is a paged, newest-first list. Each entry shows a compact relative date (`Just now`, `1h ago`, `Yesterday`, `1 week ago`), audio duration, detected language when available, and a two-line normalized text preview.
- The right pane shows the selected entry. It includes its relative date, exact local creation timestamp, duration, language when available, full scrollable transcription text, and a Copy text action. Copying uses the browser clipboard API and gives brief inline confirmation.

The newest entry is selected when History opens. When no entry exists, the detail panel provides an empty state explaining that successfully completed transcriptions appear here. Deleting the selected entry selects the adjacent following row (or the preceding row when the deleted item was last); when none remains, the empty state appears.

Each entry offers a delete action with confirmation. A Clear all history action in the sidebar footer requires a stronger confirmation before permanently removing all stored transcriptions.

## Local storage model

All history data lives below Aywa's local app-data directory and is never uploaded.

```text
<app-data>/history/
  index.json
  transcripts/
    <entry-id>.txt
```

`index.json` is a versioned metadata index. It contains lightweight records only:

- stable entry ID;
- creation time as UNIX epoch milliseconds;
- source audio duration in milliseconds;
- language code or `null` when it is unavailable;
- normalized, length-limited preview;
- text character count.

Each `<entry-id>.txt` file stores the complete UTF-8 transcription. Raw recordings remain temporary and are deleted under the existing recording lifecycle.

## Backend contract and lifecycle

Create an isolated Rust history module responsible for storage, validation, and cleanup. Its responsibilities are:

- `save`: write a new transcript and add matching metadata to the index;
- `list`: return a newest-first page of metadata from a non-negative offset and fixed limit;
- `read`: return one transcript body by ID;
- `delete`: remove one body file and its index record;
- `clear`: remove all body files and reset the index;
- `startup_repair`: tolerate a missing index, discard invalid metadata records, and remove orphaned temporary files.

History writes use a temporary file plus atomic rename. The body file is committed before its index entry; the updated index is also committed through a temporary-file rename. A crash can leave an unreferenced body, but never a metadata record that points at a partially written body. Startup repair clears such temporary/orphaned files.

After the transcription worker returns a successful result, Aywa saves the result to History before moving to the completed UI state. Clipboard copying and the existing completion behavior remain unchanged. If the History save fails, transcription still completes and copies; Aywa records/logs the history failure without losing the successful transcription workflow.

Expose Tauri commands for paged metadata listing, loading one transcript body, deleting one entry, and clearing history. Input IDs must be validated against the index and never used as unconstrained filesystem paths.

## Performance and memory limits

The frontend requests metadata only, initially one page of 50 entries. It requests later pages only as the user scrolls near the end of the list. It does not preload transcript bodies. Relative-date labels are recomputed at least once per minute while History is visible.

Selecting an entry requests only that text file. The UI retains at most the currently selected transcript body; it releases the previous body whenever the selection changes, is deleted, or the History component unmounts. This makes list size and transcript length independent of normal UI memory use.

The initial implementation maintains no search index because v1 does not include search.

## Error handling

- If a listed body file is missing or unreadable, show an inline unavailable state and offer deletion of its stale entry.
- Failed list, read, copy, delete, and clear operations show an inline error without clearing working UI state.
- Clear all and per-entry deletion require confirmation and only update the local UI after the backend succeeds.
- Relative dates update while the History view is open; exact timestamps remain visible for precision and accessibility.

## Verification

Rust tests cover index migration/defaults, atomic save behavior, newest-first ordering, pagination boundaries, body reads, individual deletion, clear all, invalid IDs, and missing/corrupt/orphaned data recovery.

Frontend tests cover relative-date formatting, initial/empty/loading/error states, lazy page loading, selection and replacement of loaded transcript text, scrollable content, copy confirmation/failure, unavailable-body state, and delete/clear confirmation flows.

Manual smoke checks verify that a completed recording appears after reopening the app, that History preserves text but not audio files, that a long library remains responsive, and that clearing History removes the local stored transcript content.
