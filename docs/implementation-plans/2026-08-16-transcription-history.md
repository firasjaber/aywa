# Transcription History Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (\`- [ ]\`) syntax for tracking.

**Goal:** Persist every successful transcription locally and expose a memory-efficient History tab for browsing, reading, copying, and deleting it.

**Architecture:** A Rust \`history\` module owns a versioned metadata index and one UTF-8 text file per transcript under the existing app-data root. Tauri commands expose paged metadata and on-demand text reads; the Svelte History component owns at most one selected transcript body.

**Tech Stack:** Rust, Serde/serde_json, Tauri 2, Svelte 5, TypeScript, Vitest, Svelte Check.

---

## File structure

- \`src-tauri/src/history.rs\` — atomic local persistence, validation, repair, and Rust unit tests.
- \`src-tauri/src/audio.rs\` — source-audio duration calculation.
- \`src-tauri/src/lib.rs\` — non-fatal save after successful transcription and app-level history operations.
- \`src-tauri/src/main.rs\` — Tauri command wrappers and startup repair.
- \`src/lib/stores/history.ts\` — typed Tauri DTOs and invocations.
- \`src/lib/components/historyView.ts\` — pure date, duration, and selection helpers.
- \`src/lib/components/historyView.test.ts\` — deterministic frontend unit tests.
- \`src/lib/components/History.svelte\` — lazy two-pane interface and scoped styles.
- \`src/lib/components/Settings.svelte\`, \`src/lib/components/settingsView.ts\` — History tab integration.
- \`package.json\`, \`package-lock.json\`, \`vitest.config.ts\` — frontend test runner.
- \`docs/USER_GUIDE.md\` — text-only local storage and deletion documentation.

### Task 1: Build a durable, disk-backed History repository

**Files:**

- Create: \`src-tauri/src/history.rs\`
- Modify: \`src-tauri/src/lib.rs:1-25\`

- [ ] **Step 1: Add failing save/list tests**

Start \`history.rs\` with the following tests and compile them before creating the repository implementation.

\`\`\`rust
#[test]
fn save_lists_metadata_without_loading_transcript_bodies() {
    let directory = tempfile::tempdir().unwrap();
    let repository = HistoryRepository::new(directory.path().join("history"));

    let saved = repository.save_at("First line\nsecond line", 1_000, None, 20).unwrap();
    let page = repository.list(0, 50).unwrap();

    assert_eq!(page.entries, vec![saved]);
    assert_eq!(page.next_offset, None);
    assert_eq!(
        std::fs::read_to_string(repository.transcript_path_for_test(&saved.id)).unwrap(),
        "First line\nsecond line"
    );
}

#[test]
fn list_is_newest_first_and_respects_offsets() {
    let directory = tempfile::tempdir().unwrap();
    let repository = HistoryRepository::new(directory.path().join("history"));
    let old = repository.save_at("old", 500, None, 1).unwrap();
    let new = repository.save_at("new", 600, None, 2).unwrap();

    assert_eq!(repository.list(0, 1).unwrap().entries, vec![new.clone()]);
    assert_eq!(repository.list(0, 1).unwrap().next_offset, Some(1));
    assert_eq!(repository.list(1, 1).unwrap().entries, vec![old]);
}
\`\`\`

- [ ] **Step 2: Verify the test fails**

Run from \`src-tauri\`:

\`\`\`powershell
cargo test history::tests::save_lists_metadata_without_loading_transcript_bodies
\`\`\`

Expected: compilation fails because \`HistoryRepository\` is undefined.

- [ ] **Step 3: Define metadata-only public types**

Implement the following types. The full text must never appear in \`HistoryEntryMetadata\`.

\`\`\`rust
pub const HISTORY_SCHEMA_VERSION: u32 = 1;
pub const MAX_PAGE_SIZE: usize = 50;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct HistoryEntryMetadata {
    pub id: String,
    pub created_at_epoch_ms: u64,
    pub duration_ms: u64,
    pub language: Option<String>,
    pub preview: String,
    pub character_count: usize,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryPage {
    pub entries: Vec<HistoryEntryMetadata>,
    pub next_offset: Option<usize>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryTranscript {
    pub id: String,
    pub text: String,
}
\`\`\`

Use a private \`HistoryIndex { version: u32, entries: Vec<HistoryEntryMetadata> }\`. The epoch-millisecond field is the canonical storage representation; the frontend formats it as relative and exact local dates.

- [ ] **Step 4: Implement the repository boundary**

Implement this exact public API:

\`\`\`rust
impl HistoryRepository {
    pub fn new(root: PathBuf) -> Self;
    pub fn save(&self, text: &str, duration_ms: u64, language: Option<String>) -> Result<HistoryEntryMetadata, String>;
    pub fn list(&self, offset: usize, limit: usize) -> Result<HistoryPage, String>;
    pub fn read(&self, id: &str) -> Result<HistoryTranscript, String>;
    pub fn delete(&self, id: &str) -> Result<(), String>;
    pub fn clear(&self) -> Result<(), String>;
    pub fn startup_repair(&self) -> Result<(), String>;
}
\`\`\`

Create \`root/transcripts\` on save. Build the preview by collapsing whitespace and taking the first 160 Unicode scalar values. Generate IDs from UNIX nanoseconds. Clamp list limits to \`MAX_PAGE_SIZE\`, sort metadata descending by \`created_at_epoch_ms\`, and return \`next_offset: None\` at the end.

Write each body to \`<id>.txt.tmp\`, rename it to \`<id>.txt\`, then serialize and atomically replace \`index.json\` through \`index.json.tmp\`. Read and delete only an ID found in the loaded index; never append an untrusted ID to a path. Return \`History entry not found\` for unknown IDs.

- [ ] **Step 5: Add recovery and destructive-operation tests**

Add concrete tests for deletion, clearing, missing bodies, bad index records, and stale temporary files:

\`\`\`rust
#[test]
fn delete_removes_the_entry_and_its_body() {
    let directory = tempfile::tempdir().unwrap();
    let repository = HistoryRepository::new(directory.path().join("history"));
    let entry = repository.save_at("delete me", 10, None, 1).unwrap();

    repository.delete(&entry.id).unwrap();

    assert!(repository.list(0, 50).unwrap().entries.is_empty());
    assert!(!repository.transcript_path_for_test(&entry.id).exists());
}

#[test]
fn clear_removes_every_body_and_resets_the_index() {
    let directory = tempfile::tempdir().unwrap();
    let repository = HistoryRepository::new(directory.path().join("history"));
    repository.save_at("one", 10, None, 1).unwrap();
    repository.save_at("two", 20, None, 2).unwrap();

    repository.clear().unwrap();

    assert!(repository.list(0, 50).unwrap().entries.is_empty());
    assert!(!repository.root.join("transcripts").exists());
}
\`\`\`

Also write the literal text \`not json\` to \`index.json\` and assert \`startup_repair\` returns an empty valid index, write a \`.tmp\` body and assert repair removes it, and assert a missing selected body causes \`read\` to return an error while other list records remain usable.

- [ ] **Step 6: Register and validate the module**

Add \`pub mod history;\` in \`src-tauri/src/lib.rs\`.

Run:

\`\`\`powershell
cd src-tauri
cargo test history::tests
cargo fmt --check
\`\`\`

Expected: repository tests pass and formatting is clean.

- [ ] **Step 7: Commit**

\`\`\`powershell
git add src-tauri/src/history.rs src-tauri/src/lib.rs
git commit -m "feat: persist local transcription history"
\`\`\`

### Task 2: Save each successful transcription and publish command boundaries

**Files:**

- Modify: \`src-tauri/src/audio.rs:15-20,107-129,270-324\`
- Modify: \`src-tauri/src/lib.rs:128-174\`
- Modify: \`src-tauri/src/main.rs:9-18,62-185,246-265\`

- [ ] **Step 1: Add the source-audio duration test**

Add to the existing \`audio.rs\` test module:

\`\`\`rust
#[test]
fn duration_ms_uses_mono_sample_count_and_sample_rate() {
    assert_eq!(duration_ms(48_000, 48_000), 1_000);
    assert_eq!(duration_ms(24_000, 48_000), 500);
}
\`\`\`

- [ ] **Step 2: Carry source-audio duration with the recording**

Add the helper and field below, calculating it from \`mono_samples\` after downmixing:

\`\`\`rust
pub fn duration_ms(sample_count: usize, sample_rate: u32) -> u64 {
    (sample_count as u64).saturating_mul(1_000) / u64::from(sample_rate.max(1))
}

pub struct RecordingResult {
    pub path: PathBuf,
    pub sample_rate: u32,
    pub sample_count: usize,
    pub duration_ms: u64,
}
\`\`\`

Do not reuse \`TranscriptionResult.duration_ms\`: that value measures worker execution time rather than audio length.

- [ ] **Step 3: Save History without changing successful clipboard behavior**

Add \`history_dir(app) -> Result<PathBuf, String>\` in \`lib.rs\`, returning \`app_data_dir(app)?.join("history")\`. Add app-level \`list_history\`, \`read_history_entry\`, \`delete_history_entry\`, \`clear_history\`, and \`repair_history_on_startup\` wrappers that delegate to \`HistoryRepository\`.

In the success branch of \`stop_recording\`, save before the existing clipboard copy:

\`\`\`rust
if let Err(error) = history::HistoryRepository::new(history_dir(app)?).save(
    &transcription.text,
    result.duration_ms,
    None,
) {
    diagnostics::log(format!("[history] save failed: {error}"));
}
clipboard::copy_text(&transcription.text)?;
\`\`\`

The \`None\` language is intentional: no current worker response provides reliable language metadata. The UI must omit the field, not invent an English value. The history failure stays non-fatal.

- [ ] **Step 4: Repair on app startup and expose commands**

In \`main.rs\`, call \`repair_history_on_startup\` next to \`cleanup_recordings_on_startup\`; log an error prefixed \`[history]\` but continue startup.

Add these wrappers and their delete/clear counterparts, then add all four to \`tauri::generate_handler!\`:

\`\`\`rust
#[tauri::command]
fn list_history(app: AppHandle, offset: usize) -> Result<aywa_lib::history::HistoryPage, String> {
    aywa_lib::list_history(&app, offset)
}

#[tauri::command]
fn read_history_entry(
    app: AppHandle,
    id: String,
) -> Result<aywa_lib::history::HistoryTranscript, String> {
    aywa_lib::read_history_entry(&app, &id)
}
\`\`\`

The other exact command names are \`delete_history_entry(app, id)\` and \`clear_history(app)\`. Do not add browser filesystem capabilities; all filesystem work stays in Rust.

- [ ] **Step 5: Validate and commit**

\`\`\`powershell
cd src-tauri
cargo test
cargo fmt --check
git add src-tauri/src/audio.rs src-tauri/src/lib.rs src-tauri/src/main.rs
git commit -m "feat: save completed transcriptions to history"
\`\`\`

Expected: all Rust tests pass and only intended files are staged.

### Task 3: Add typed frontend data access and testable view helpers

**Files:**

- Modify: \`package.json\`, \`package-lock.json\`
- Create: \`vitest.config.ts\`
- Create: \`src/lib/stores/history.ts\`
- Create: \`src/lib/components/historyView.ts\`
- Create: \`src/lib/components/historyView.test.ts\`

- [ ] **Step 1: Add a frontend test command**

Install Vitest as a dev dependency, add \`"test": "vitest run"\` to \`scripts\`, and create:

\`\`\`ts
// vitest.config.ts
import { defineConfig } from 'vitest/config';

export default defineConfig({
    test: { environment: 'node', include: ['src/**/*.test.ts'] }
});
\`\`\`

- [ ] **Step 2: Write deterministic helper tests**

Create \`historyView.test.ts\`:

\`\`\`ts
import { describe, expect, it } from 'vitest';
import { formatHistoryDuration, formatRelativeHistoryTime, nextSelectedId } from './historyView';

describe('History view helpers', () => {
    const now = Date.UTC(2026, 7, 16, 12, 0, 0);

    it('formats compact relative dates', () => {
        expect(formatRelativeHistoryTime(now - 20_000, now)).toBe('Just now');
        expect(formatRelativeHistoryTime(now - 3_600_000, now)).toBe('1h ago');
        expect(formatRelativeHistoryTime(now - 7 * 86_400_000, now)).toBe('1 week ago');
    });

    it('formats audio duration and selection replacement', () => {
        expect(formatHistoryDuration(65_000)).toBe('1:05');
        expect(nextSelectedId(['new', 'middle', 'old'], 'middle')).toBe('old');
        expect(nextSelectedId(['only'], 'only')).toBeNull();
    });
});
\`\`\`

Add expectations for plural minutes/hours/days and \`h:mm:ss\` duration.

- [ ] **Step 3: Implement pure helper behavior**

Implement \`formatRelativeHistoryTime(createdAtEpochMs, now = Date.now())\` with these thresholds: under one minute is \`Just now\`; under one hour uses whole minutes; under 24 hours whole hours; under seven days whole days; under 30 days whole weeks; older values use \`Intl.DateTimeFormat\` with short month, numeric day, and numeric year.

Implement \`formatHistoryDuration\` as \`m:ss\` below one hour and \`h:mm:ss\` otherwise. Implement \`nextSelectedId\` to select the row after the deleted row, use the preceding row if the deleted row was last, and return \`null\` if no other row exists.

- [ ] **Step 4: Implement typed invoke wrappers**

Create \`src/lib/stores/history.ts\`:

\`\`\`ts
import { invoke } from '@tauri-apps/api/core';

export type HistoryEntry = {
    id: string;
    createdAtEpochMs: number;
    durationMs: number;
    language: string | null;
    preview: string;
    characterCount: number;
};
export type HistoryPage = { entries: HistoryEntry[]; nextOffset: number | null };
export type HistoryTranscript = { id: string; text: string };

export const listHistory = (offset: number) => invoke<HistoryPage>('list_history', { offset });
export const readHistoryEntry = (id: string) => invoke<HistoryTranscript>('read_history_entry', { id });
export const deleteHistoryEntry = (id: string) => invoke<void>('delete_history_entry', { id });
export const clearHistory = () => invoke<void>('clear_history');
\`\`\`

Do not add a global store of transcript text.

- [ ] **Step 5: Run checks and commit**

\`\`\`powershell
npm test
npm run check
git add package.json package-lock.json vitest.config.ts src/lib/stores/history.ts src/lib/components/historyView.ts src/lib/components/historyView.test.ts
git commit -m "feat: add typed history client helpers"
\`\`\`

Expected: tests pass and Svelte Check reports zero errors and warnings.

### Task 4: Create the lazy two-pane History interface

**Files:**

- Create: \`src/lib/components/History.svelte\`
- Modify: \`src/lib/components/Settings.svelte:1-30,178-265,441-900\`
- Modify: \`src/lib/components/settingsView.ts:6\`

- [ ] **Step 1: Add the History tab**

Change the type to:

\`\`\`ts
export type SettingsTab = 'app' | 'history' | 'debug';
\`\`\`

Import \`History\` into \`Settings.svelte\`, add a History tab button between App and Debug using \`aria-controls="history-panel"\`, and render \`<History />\` only when \`activeTab === 'history'\`. Unmounting frees the selected text body.

- [ ] **Step 2: Implement loading, selection, and stale-request protection**

In \`History.svelte\`, use local Svelte state:

\`\`\`ts
let entries = $state<HistoryEntry[]>([]);
let nextOffset = $state<number | null>(0);
let selectedId = $state<string | null>(null);
let selectedText = $state<string | null>(null);
let loading = $state(true);
let loadingMore = $state(false);
let error = $state<string | null>(null);
\`\`\`

On mount, fetch \`listHistory(0)\`, store metadata, then select the newest entry. In \`selectEntry(id)\`, set \`selectedId = id\` and \`selectedText = null\` before requesting its body. Assign a response only when \`selectedId === id\`, so a delayed previous response cannot replace the currently selected text.

- [ ] **Step 3: Lazily append metadata pages**

Put a zero-height sentinel below the list and use \`IntersectionObserver\`. When the sentinel intersects, \`nextOffset !== null\`, and \`loadingMore\` is false, call \`listHistory(nextOffset)\`, append its metadata, and replace \`nextOffset\`. Disconnect on destroy. Never call \`readHistoryEntry\` as part of pagination.

- [ ] **Step 4: Render all required states and actions**

Render initial loading, no-history, metadata-list error with Retry, selected-entry loading, missing-body error with Delete entry, and full detail states. Full detail includes a semantic \`<time datetime={new Date(entry.createdAtEpochMs).toISOString()}>\`, compact relative date, exact local date/time, formatted audio duration, optional language, and a scrollable \`<pre>\` with \`white-space: pre-wrap\`.

Use \`navigator.clipboard.writeText(selectedText)\` for Copy text and show \`Copied\` only after success. Use \`window.confirm\` before individual deletion and clear all. After a deletion succeeds, remove only that metadata row, set \`selectedText = null\`, derive the replacement using \`nextSelectedId\`, then load exactly that replacement body. After a successful clear, reset entries, cursor, selection, and text to empty values.

- [ ] **Step 5: Apply responsive, accessible component styles**

Create a fixed-height \`.history-layout\` grid with an independently scrolling sidebar and \`minmax(0, 1fr)\` detail pane. Use visible selected-row contrast, clamp previews to two lines, distinguish destructive actions in red, and make the transcript pane scrollable with a readable line height.

Include:

\`\`\`css
@media (max-width: 38rem) {
    .history-layout { grid-template-columns: 1fr; }
    .history-list { max-height: 15rem; }
}
\`\`\`

Do not add animated insertion or list transitions; this honors reduced-motion preferences without extra behavior.

- [ ] **Step 6: Validate and commit**

\`\`\`powershell
npm test
npm run check
npm run tauri -- dev
\`\`\`

Manually create two transcriptions. Confirm newest-first list order, relative dates, one loaded body at a time, scrollable text, Copy feedback, delete/clear confirmations, and persistence after restart. Confirm \`runtime/recordings\` holds no retained History audio.

\`\`\`powershell
git add src/lib/components/History.svelte src/lib/components/Settings.svelte src/lib/components/settingsView.ts
git commit -m "feat: add transcription history tab"
\`\`\`

### Task 5: Document the privacy model and run release verification

**Files:**

- Modify: \`docs/USER_GUIDE.md\`
- Verify: all files from Tasks 1-4

- [ ] **Step 1: Document local History behavior**

Add a History section stating that every successful transcription is saved automatically on the device; only text and metadata are retained; recordings remain temporary; people can browse, copy, delete a single entry, or permanently clear all history; and Aywa uploads neither recordings nor history.

- [ ] **Step 2: Run all automated checks**

\`\`\`powershell
npm test
npm run check
cd src-tauri
cargo fmt --check
cargo test
\`\`\`

Expected: all commands exit 0.

- [ ] **Step 3: Smoke-test recovery and data removal**

Create an entry, restart Aywa, and confirm it opens. With the app stopped, rename the matching transcript \`.txt\` file, restart, and confirm its unavailable-body state does not hide other entries. Restore the file, then use Clear all and confirm the History empty state and deleted transcript files.

- [ ] **Step 4: Commit documentation and check handoff**

\`\`\`powershell
git diff --check
git add docs/USER_GUIDE.md
git commit -m "docs: explain local transcription history"
git status --short
\`\`\`

Expected: no whitespace errors and no unintended staged files.

## Plan self-review

- **Spec coverage:** Tasks 1-2 cover local text-only storage, atomic writes, repair, automatic non-fatal saving, page APIs, and audio non-retention. Tasks 3-4 cover bounded memory, relative/exact dates, browsing, copy, deletion, confirmation, empty/loading/error handling, and responsive layout. Task 5 verifies privacy, persistence, missing-body recovery, and clear-all deletion.
- **Placeholder scan:** All requirements are actionable: public types, command names, timing thresholds, test commands, and commit boundaries are concrete.
- **Type consistency:** Rust Serde camelCase fields match \`HistoryEntry\`, \`HistoryPage\`, and \`HistoryTranscript\`. The frontend calls \`list_history(offset)\`, \`read_history_entry(id)\`, \`delete_history_entry(id)\`, and \`clear_history()\`, matching the required command wrappers exactly.
