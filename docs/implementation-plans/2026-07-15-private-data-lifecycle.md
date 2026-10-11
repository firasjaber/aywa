# Private Data Lifecycle Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Delete temporary microphone WAVs reliably, bound redacted diagnostics to 1 GB of per-launch logs, and enable a previewed safe issue report in Settings.

**Architecture:** A new pure `recordings` module owns temporary WAV cleanup and startup orphan cleanup. The diagnostics module owns session-file retention, safe event redaction, and report formatting; `lib.rs` builds a typed report snapshot, and the existing Tauri/Svelte layers display and copy that exact preview.

**Tech Stack:** Rust 2021, Tauri 2, Svelte 5, TypeScript, `tempfile`, Cargo tests, Svelte Check.

---

## File structure

- Create: `src-tauri/src/recordings.rs` — RAII WAV deletion and orphan cleanup.
- Modify: `src-tauri/src/diagnostics.rs` — session logs, 1 GB retention, redaction, and report formatter.
- Modify: `src-tauri/src/lib.rs` and `src-tauri/src/main.rs` — recording/report orchestration and Tauri command.
- Modify: `src-tauri/src/audio.rs`, `src-tauri/src/transcribe.rs` — remove path-bearing diagnostic events.
- Modify: `src/lib/stores/modelManager.ts`, `src/lib/components/Settings.svelte` — typed preview and copy UI.
- Modify: `README.md`, `docs/USER_GUIDE.md`, `docs/TROUBLESHOOTING.md`, `docs/SUPPORT.md`, `docs/PUBLIC_RELEASE_AUDIT.md` — policy documentation.

### Task 1: Own every completed WAV until deletion

**Files:**
- Create: `src-tauri/src/recordings.rs`
- Modify: `src-tauri/src/lib.rs:1-160`
- Test: `src-tauri/src/recordings.rs` inline tests

- [ ] **Step 1: Write failing cleanup tests**

```rust
#[test]
fn temporary_recording_deletes_its_wav_on_drop() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("recording-1.wav");
    std::fs::write(&path, b"wav").unwrap();
    drop(TemporaryRecording::new(path.clone()));
    assert!(!path.exists());
}

#[test]
fn cleanup_abandoned_recordings_removes_only_wav_files() {
    let directory = tempfile::tempdir().unwrap();
    let wav = directory.path().join("recording-1.wav");
    let text = directory.path().join("keep.txt");
    std::fs::write(&wav, b"wav").unwrap();
    std::fs::write(&text, b"keep").unwrap();
    assert_eq!(cleanup_abandoned_recordings(directory.path()).unwrap(), 1);
    assert!(!wav.exists());
    assert!(text.exists());
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test --manifest-path src-tauri/Cargo.toml recordings::tests -- --nocapture`

Expected: FAIL because `recordings` and `TemporaryRecording` do not exist.

- [ ] **Step 3: Implement the minimal ownership API**

```rust
pub struct TemporaryRecording { path: PathBuf }

impl TemporaryRecording {
    pub fn new(path: PathBuf) -> Self { Self { path } }
}

impl Drop for TemporaryRecording {
    fn drop(&mut self) { let _ = fs::remove_file(&self.path); }
}
```

Add `cleanup_abandoned_recordings(dir: &Path) -> Result<usize, String>`: return zero for a missing directory, enumerate regular files only, delete only a case-insensitive `.wav` extension, and return the deleted count. Export the module in `lib.rs`. Immediately after `session.stop` returns a `RecordingResult`, bind `let _temporary_recording = TemporaryRecording::new(result.path.clone());` before `transcribe_file`; do not release that guard before all subsequent result paths finish.

- [ ] **Step 4: Run the targeted tests**

Run: `cargo test --manifest-path src-tauri/Cargo.toml recordings::tests -- --nocapture`

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/recordings.rs src-tauri/src/lib.rs
git commit -m "fix: delete temporary recordings"
```

### Task 2: Make diagnostics per-session, bounded, and safe

**Files:**
- Modify: `src-tauri/src/diagnostics.rs:1-310`
- Modify: `src-tauri/src/audio.rs:20-140`
- Modify: `src-tauri/src/transcribe.rs:230-650`
- Test: `src-tauri/src/diagnostics.rs` inline tests

- [ ] **Step 1: Write failing policy tests**

```rust
#[test]
fn redact_sensitive_text_replaces_windows_paths_and_recording_names() {
    assert_eq!(redact_sensitive_text(r"C:\Users\Ava\runtime\recordings\recording-7.wav"), "<redacted-path>");
}

#[test]
fn evict_oldest_session_logs_until_total_is_within_cap() {
    let directory = tempfile::tempdir().unwrap();
    write_session(directory.path(), "aywa-session-001.log", 6);
    write_session(directory.path(), "aywa-session-002.log", 6);
    assert_eq!(enforce_log_cap(directory.path(), 10, None).unwrap(), 6);
    assert!(!directory.path().join("aywa-session-001.log").exists());
}

#[test]
fn report_excludes_paths_and_raw_output() {
    let report = build_issue_report(&sample_report_input());
    assert!(report.preview.contains("Not included"));
    assert!(!report.preview.contains("C:\\Users"));
    assert!(!report.preview.contains("recording-"));
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test --manifest-path src-tauri/Cargo.toml diagnostics::tests -- --nocapture`

Expected: FAIL because the retention, redaction, and report APIs do not exist.

- [ ] **Step 3: Implement log policy**

Define `MAX_MANAGED_LOG_BYTES: u64 = 1024 * 1024 * 1024`. Create a distinct `aywa-session-<epoch-ms>-<process-id>.log` per initialization; only files matching that exact prefix/suffix are managed. Before opening a new session and before a write, calculate managed bytes and remove whole oldest session files first until within cap. Never delete the active session. When the active session cannot accept more bytes, suppress further persisted writes and retain console output.

Replace path/raw-output persistence with a bounded safe event ring buffer and redaction. A safe event may contain event name, outcome, status, duration, model ID, and coarse device capability. It must never contain recordings, transcripts, paths, usernames, rendered commands, arguments, environment values, or raw stdout/stderr. Replace byte slicing in `summarize` with a valid `char_indices` boundary.

- [ ] **Step 4: Implement deterministic report formatting**

```rust
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IssueReport { pub preview: String }

pub struct IssueReportInput {
    pub app_version: String,
    pub operating_system: String,
    pub selected_model_id: String,
    pub loaded_model_id: Option<String>,
    pub execution_device: Option<String>,
    pub diagnostics_events: Vec<String>,
}
```

`build_issue_report` must format the listed fields and sanitized in-memory event tail, ending with: “Not included: recordings, transcripts, file paths, usernames, command arguments, or raw subprocess output.” It must not read raw log files.

- [ ] **Step 5: Run diagnostics tests**

Run: `cargo test --manifest-path src-tauri/Cargo.toml diagnostics::tests -- --nocapture`

Expected: PASS for redaction, Unicode safety, oldest-first eviction, active-log protection, cap suppression, and report exclusions.

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/diagnostics.rs src-tauri/src/audio.rs src-tauri/src/transcribe.rs
git commit -m "fix: bound and redact diagnostics"
```

### Task 3: Wire startup cleanup and safe report preview

**Files:**
- Modify: `src-tauri/src/lib.rs:80-230`
- Modify: `src-tauri/src/main.rs:80-245`
- Test: `src-tauri/src/lib.rs` inline tests

- [ ] **Step 1: Write a failing snapshot test**

```rust
#[test]
fn issue_report_uses_model_snapshot_without_log_path() {
    let report = issue_report_from_snapshot(sample_snapshot());
    assert!(report.preview.contains("Selected model"));
    assert!(!report.preview.contains("diagnosticsLogPath"));
}
```

- [ ] **Step 2: Run it to verify failure**

Run: `cargo test --manifest-path src-tauri/Cargo.toml issue_report_uses_model_snapshot_without_log_path -- --nocapture`

Expected: FAIL because `issue_report_from_snapshot` does not exist.

- [ ] **Step 3: Add startup and command wiring**

Add `cleanup_recordings_on_startup(app)` in `lib.rs`; it resolves `recordings_dir(app)`, invokes orphan cleanup, and logs only deletion count or sanitized failure category. Call it in `main.rs` setup after diagnostics initialization and before hotkey registration.

Add `preview_issue_report(app, state) -> diagnostics::IssueReport`, obtaining `list_models(app, state)` and mapping only model IDs, OS, runtime/device state, app version, and safe events into `IssueReportInput`. Add this command to `main.rs`:

```rust
#[tauri::command]
fn preview_issue_report(app: AppHandle, state: State<'_, ModelManagerState>) -> aywa_lib::diagnostics::IssueReport {
    aywa_lib::preview_issue_report(&app, state.inner())
}
```

Register it in `tauri::generate_handler![]`. Do not add a raw-log read command.

- [ ] **Step 4: Run the test**

Run: `cargo test --manifest-path src-tauri/Cargo.toml issue_report_uses_model_snapshot_without_log_path -- --nocapture`

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/lib.rs src-tauri/src/main.rs
git commit -m "feat: expose safe issue report preview"
```

### Task 4: Replace the disabled Settings action

**Files:**
- Modify: `src/lib/stores/modelManager.ts:1-170`
- Modify: `src/lib/components/Settings.svelte:1-350,760-900`
- Test: `npm run check`

- [ ] **Step 1: Add the typed invoke**

```ts
export type IssueReport = { preview: string };
export const previewIssueReport = () => invoke<IssueReport>('preview_issue_report');
```

- [ ] **Step 2: Add preview-first UI state**

In `Settings.svelte`, define `issueReportPreview: string | null`, `reportLoading`, and `reportCopied`. `generateIssueReport` awaits `previewIssueReport()` and exposes errors through existing `actionError`. `copyIssueReport` calls `navigator.clipboard.writeText(issueReportPreview)`, only reports success after it resolves, and never regenerates the payload while copying.

- [ ] **Step 3: Render the new controls**

```svelte
<p class="diagnostics-policy">Recordings are deleted after transcription. Local session logs are capped at 1 GB total.</p>
<button class="primary-action" type="button" onclick={generateIssueReport} disabled={reportLoading}>
  {reportLoading ? 'Preparing report…' : 'Preview issue report'}
</button>
{#if issueReportPreview}
  <label for="issue-report-preview">Review before copying</label>
  <textarea id="issue-report-preview" class="report-preview" readonly value={issueReportPreview}></textarea>
  <button class="primary-action" type="button" onclick={copyIssueReport}>Copy reviewed report</button>
{/if}
```

Add fixed-height scrollable preview CSS and explicit raw-log copy warning. Remove the disabled “Coming later” button.

- [ ] **Step 4: Verify**

Run: `npm run check`

Expected: PASS with zero errors and warnings.

- [ ] **Step 5: Commit**

```bash
git add src/lib/stores/modelManager.ts src/lib/components/Settings.svelte
git commit -m "feat: preview safe issue reports"
```

### Task 5: Update privacy/support documentation

**Files:**
- Modify: `README.md`
- Modify: `docs/USER_GUIDE.md`
- Modify: `docs/TROUBLESHOOTING.md`
- Modify: `docs/SUPPORT.md`
- Modify: `docs/PUBLIC_RELEASE_AUDIT.md`

- [ ] **Step 1: Add identical factual policy language**

State that WAVs exist only during local transcription and are deleted after use; WAVs left by interruption are cleared on next startup. State that logs are local, one session per launch, capped at 1 GB total, and delete oldest sessions first. State that the report excludes recordings, transcripts, paths, usernames, command arguments, and raw subprocess output; opened raw logs require review before sharing.

- [ ] **Step 2: Mark only these audit items resolved**

Replace the active recording and diagnostics/issue-report findings in `PUBLIC_RELEASE_AUDIT.md` with a dated resolution note. Leave all unrelated release findings unchanged.

- [ ] **Step 3: Search stale wording**

Run: `rg -n "retained indefinitely|Copy issue report.*Coming later|debug log path=|saved recording to" README.md docs src-tauri/src src/lib`

Expected: no production stale retention text, disabled-action text, or persisted path-bearing diagnostic messages.

- [ ] **Step 4: Commit**

```bash
git add README.md docs/USER_GUIDE.md docs/TROUBLESHOOTING.md docs/SUPPORT.md docs/PUBLIC_RELEASE_AUDIT.md
git commit -m "docs: explain private data lifecycle"
```

### Task 6: Run full privacy regression verification

**Files:**
- Modify: only files necessary to correct verification failures

- [ ] **Step 1: Run Rust tests**

Run: `cargo test --manifest-path src-tauri/Cargo.toml`

Expected: PASS, including cleanup, retention/redaction, report, audio, and worker tests.

- [ ] **Step 2: Run frontend checks**

Run: `npm run check; npm run build`

Expected: both PASS without Svelte or TypeScript errors.

- [ ] **Step 3: Run Python protocol tests**

Run: `python -m pytest src-tauri/python/tests -v`

Expected: PASS.

- [ ] **Step 4: Sweep for prohibited production diagnostics**

Run: `rg -n "audio_path=|python_exe=|model_path=|worker_script=|saved recording to|render_command\(" src-tauri/src`

Expected: any remaining match is a local variable or test; no production persisted diagnostic writes a path, command, or raw output.

- [ ] **Step 5: Commit verification corrections**

```bash
git add src-tauri/src src/lib README.md docs
git commit -m "test: verify private data lifecycle"
```

## Plan self-review

- Spec coverage: Tasks 1 and 3 cover deletion and startup recovery; Task 2 covers session logs, cap, redaction, and reporting; Task 4 covers preview-before-copy; Task 5 documents policy; Task 6 tests acceptance criteria.
- Placeholder scan: all changes specify concrete APIs, markup, test assertions, commands, and expected outcomes.
- Type consistency: `IssueReport { preview }` and `preview_issue_report` are the shared backend/frontend contract; `TemporaryRecording` owns each WAV after successful capture.

