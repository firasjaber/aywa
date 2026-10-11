# Aywa Private Data Lifecycle Design

## Goal

Make microphone recordings temporary, bound diagnostic storage to 1 GB of per-launch logs, and let users inspect a redacted issue report before copying it for support.

## Scope

This design addresses GitHub issue #3. It changes only the recording lifecycle, diagnostics storage and content, issue-report generation, Settings diagnostics UI, and related documentation. It does not add cloud telemetry, audio-history retention, or automated report submission.

## Recording lifecycle

WAV recordings remain an implementation detail of the transcription pipeline. They are stored under `runtime/recordings` only while the active transcription request needs them.

After `RecorderSession::stop` successfully creates a WAV, `stop_recording` constructs an ownership-based cleanup guard for that exact path before transitioning to processing. The guard removes the file in its `Drop` implementation, so it runs for every return path: successful transcription, worker failure, clipboard failure, paste failure, and state-transition failure. Cleanup is best effort: a deletion failure is recorded as a safe event without the absolute recording path, and it does not replace the user-facing transcription outcome.

At startup, before the app accepts recording, Aywa scans `runtime/recordings` and removes regular `.wav` files left by an interrupted or crashed prior session. It leaves non-WAV files untouched. The scan tolerates a missing directory; individual deletion errors do not stop startup and are logged as sanitized events.

There is no user setting to retain recordings. Product documentation states this plainly: audio is held temporarily for local transcription and deleted after use; a power loss or forced termination can leave an abandoned WAV only until the next startup cleanup.

## Diagnostics storage and event policy

Each app launch writes to one new session log under `logs/`, named with a timestamp and collision-resistant session identifier. A stable, human-readable current-session filename may be exposed internally for the existing “open log” action, but a new launch must never append to a prior session.

Before a session log is opened and before each write that can grow it, diagnostics enforcement calculates the total size of managed session-log files. If admitting the current session would exceed 1 GB, it deletes complete oldest session logs first until the managed total is within the cap. The active log is never deleted while it is open. If one session itself reaches the 1 GB limit, further file writes are suppressed for that launch while console diagnostics can continue; the app remains usable. Only files matching Aywa’s managed session-log naming pattern are eligible for deletion.

Logs are minimal, structured operational events. Permitted fields are event name, success/failure category, elapsed duration, status code, model identifier, app/runtime version, and coarse device capability (for example `cpu`, `cuda_available`, or `cuda_unavailable`). They must not contain:

- transcript or audio content;
- recording names or any file-system path;
- usernames or home/app-data locations;
- raw command lines, arguments, environment variables, or complete subprocess output;
- unbounded exception/worker output that could contain any of the above.

Command execution continues to return concise, user-actionable error summaries to the invoking workflow, but diagnostics logs only the command description, outcome, status, timeout, and a categorized/sanitized failure reason. A shared redaction helper replaces known paths and defensive path-like strings before persisted diagnostics or support-report data is produced. Text shortening is Unicode-safe.

## Support issue report

The diagnostics backend exposes a deterministic report builder. Its output is plaintext and contains only:

- report format version and generation time;
- Aywa and operating-system versions;
- selected/loaded model ID and runtime-profile state;
- coarse execution and CUDA readiness state;
- a bounded tail of already-sanitized diagnostic events;
- a fixed “not included” statement covering recordings, transcripts, paths, usernames, command arguments, and raw subprocess output.

The report never reads arbitrary raw logs, enumerates files, or includes the diagnostics directory path. Report building returns a data object with the preview text and copy-ready text so the UI cannot preview one payload and copy another.

## Settings experience

The existing Diagnostics tab replaces the disabled “Copy issue report” control with a generated report preview and an enabled copy action once the report can be built. The preview appears before clipboard access, uses a fixed-height scrollable text region, and is regenerated on demand so it reflects current model/runtime state. Copying sends precisely the previewed text to the clipboard and gives clear success or error feedback.

The tab states that recordings are deleted after transcription, diagnostics are local per-session files capped at 1 GB total, and the preview lists exactly what support data will be copied. The raw-log file-manager action remains available for local troubleshooting but is visually distinguished from the safe report and warns users to review raw logs before sharing them.

## Components and boundaries

- `src-tauri/src/recordings.rs` (new) owns temporary WAV cleanup and stale-recording cleanup. It has no Tauri state dependency and receives explicit paths, making it unit-testable.
- `src-tauri/src/diagnostics.rs` owns session-log naming, cap enforcement, safe event writing, path redaction, and pure report formatting. It receives report inputs as typed snapshots rather than reading UI state or arbitrary files.
- `src-tauri/src/lib.rs` orchestrates recording cleanup and exposes a Tauri command that asks diagnostics to build a support-report snapshot from existing model/runtime state.
- `src/lib/stores/modelManager.ts` adds typed invokes for preview/copy report operations.
- `src/lib/components/Settings.svelte` renders policy copy, the preview, copy feedback, and the clearly separate raw-log action.

## Error handling

Failure to remove a recording or stale WAV does not retain it silently: the app reports a sanitized diagnostic event and continues the primary user flow. Failure to create a session log leaves console output available and makes the raw-log action report that no file is available. Failure to build or copy a report leaves the preview/action in an explicit error state; it never falls back to copying raw logs.

## Tests and documentation

Rust tests cover cleanup guard behavior, cleanup on error paths, startup removal of WAV-only abandoned files, session-log ordering and cap eviction, active-log preservation, log-write suppression at cap, redaction of Windows paths and Unicode-safe truncation, safe command-event output, and report field inclusion/exclusion. Frontend checks cover preview-before-copy, matching preview/copy payload, loading/error feedback, and raw-log separation using the project’s existing frontend verification facilities.

`README.md`, `docs/USER_GUIDE.md`, `docs/TROUBLESHOOTING.md`, `docs/SUPPORT.md`, and `docs/PUBLIC_RELEASE_AUDIT.md` are updated to describe temporary recordings, per-session 1 GB diagnostics retention, safe report contents, raw-log sharing caution, and completion of this issue’s audit findings.

## Acceptance criteria

1. No completed or failed transcription leaves its WAV behind during normal process execution.
2. Startup removes abandoned WAVs from the recording directory without deleting unrelated files.
3. Diagnostics are segmented by launch and managed logs never exceed 1 GB except transiently during a single write; oldest sessions are removed first.
4. Persisted diagnostics and generated reports do not expose paths, recording identifiers, transcript content, raw command arguments, or raw subprocess output.
5. A user can review then copy a deterministic support report from Settings; the copied payload matches the preview exactly.
6. User-facing documentation accurately describes all of the above.
