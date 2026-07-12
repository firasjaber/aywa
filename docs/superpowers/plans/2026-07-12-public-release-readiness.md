# Aywa Public Release Readiness Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Audit Aywa for a Windows-only public release and deliver accurate README, setup, troubleshooting, support, and GitHub bug-reporting documentation.

**Architecture:** Treat the current worktree as the product under review, gather evidence before writing claims, and keep the audit separate from user-facing guidance. The README acts as a short navigation hub; focused guides own detailed setup, recovery, and issue-report workflows.

**Tech Stack:** Markdown, GitHub issue forms, PowerShell, Svelte 5/TypeScript, Tauri 2/Rust, Python/pytest

---

## File map

- Modify `README.md`: public project landing page and documentation hub.
- Create `docs/PUBLIC_RELEASE_AUDIT.md`: prioritized, evidence-based release-readiness findings.
- Create `docs/SETUP.md`: packaged installation and source-development setup for Windows.
- Create `docs/USER_GUIDE.md`: non-technical installation, first-launch, model, acceleration, and transcription walkthrough.
- Create `docs/TROUBLESHOOTING.md`: symptom-oriented recovery procedures.
- Create `docs/SUPPORT.md`: diagnosis, log collection, privacy review, and issue-submission workflow.
- Create `.github/ISSUE_TEMPLATE/bug_report.yml`: structured GitHub bug-report form.
- Reference `docs/superpowers/specs/2026-07-12-public-release-readiness-design.md`: approved scope and acceptance criteria.

### Task 1: Establish the audit evidence set

**Files:**
- Inspect: all tracked first-party files under `src/`, `src-tauri/`, `scripts/`, `static/`, and `docs/`
- Inspect: `package.json`, `package-lock.json`, `.gitignore`, `src-tauri/Cargo.toml`, `src-tauri/Cargo.lock`, `src-tauri/tauri.conf.json`
- Create: `docs/PUBLIC_RELEASE_AUDIT.md`

- [ ] **Step 1: Capture repository and tracked-file hygiene evidence**

Run:

```powershell
git status --short
git ls-files
git log -10 --oneline --decorate
git remote -v
```

Expected: record the dirty worktree without modifying it; identify tracked logs, generated artifacts, missing community files, and stale public-facing names.

- [ ] **Step 2: Search for public-release risk patterns**

Run:

```powershell
rg -n --hidden -g '!node_modules/**' -g '!target/**' -g '!build/**' -g '!*.lock' '(?i)(api[_-]?key|secret|token|password|private[_-]?key|authorization|bearer)'
rg -n -g '*.rs' -g '*.py' -g '*.ts' -g '*.svelte' 'TODO|FIXME|HACK|XXX|unwrap\(|expect\(|panic!|unimplemented!|todo!|unsafe\b|let _ =|except Exception|console\.|eprintln!'
rg -n -g '!*.lock' -g '!static/logo/**' 'C:\\Users\\|/Users/|/home/|localhost|http://|https://|OK\b|ok-debug|sv create'
```

Expected: produce a review queue for manual confirmation; do not report test fixtures or dependency-lock entries as defects without context.

- [ ] **Step 3: Inspect security- and privacy-sensitive flows**

Read the Tauri capabilities and CSP, diagnostic logging, download and archive extraction, runtime provisioning, subprocess invocation, microphone capture, clipboard/text insertion, hotkey lifecycle, and Python worker protocol. Trace inputs to filesystem paths, commands, logs, and UI-visible errors.

Expected: every reported security/privacy finding cites a concrete file and line and distinguishes a confirmed defect from a risk requiring validation.

- [ ] **Step 4: Inspect maintainability and public-project readiness**

Measure source-file sizes and review module boundaries, duplicated state/types, error-string handling, concurrency ownership, cleanup, test coverage, version/metadata consistency, Windows-only behavior, dependency pinning, and missing CI/community files.

Run:

```powershell
Get-ChildItem src,src-tauri,scripts -Recurse -File | Where-Object { $_.Extension -in '.rs','.py','.ts','.svelte','.ps1' } | ForEach-Object { [pscustomobject]@{ Lines = (Get-Content $_.FullName).Count; Path = $_.FullName } } | Sort-Object Lines -Descending
```

Expected: identify specific design smells and contributor friction rather than treating file length alone as a defect.

- [ ] **Step 5: Run baseline verification**

Run:

```powershell
npm.cmd run check
npm.cmd run build
cargo test --manifest-path src-tauri/Cargo.toml
python -m pytest src-tauri/python/tests -v
```

Expected: record pass/fail and exact environmental limitations. Do not install or change dependencies merely to make the audit green.

- [ ] **Step 6: Write the prioritized audit**

Create `docs/PUBLIC_RELEASE_AUDIT.md` with an executive summary, release recommendation, verification results, and Blocker/High/Medium/Low sections. Each finding uses this structure:

```markdown
### [Severity] Short finding title

- **Evidence:** `path/to/file.ext:line` and the observed behavior.
- **Impact:** Why a user, contributor, or maintainer would care.
- **Recommendation:** A bounded corrective action.
```

Include strengths worth preserving and a pre-publication checklist. Do not change application code.

### Task 2: Replace the scaffold README

**Files:**
- Modify: `README.md`
- Reference: `docs/PUBLIC_RELEASE_AUDIT.md`

- [ ] **Step 1: Write the public landing page**

Replace the scaffold with these sections in order: title and one-sentence purpose; Windows-only/maturity notice; key features; privacy and network behavior; quick start for packaged releases; everyday voice-typing workflow; documentation links; source-development summary; architecture; support; contributing status; license status.

Use `https://github.com/firasjaber/stt` for repository and issue links. State unimplemented or owner-dependent items plainly; do not fabricate release assets, screenshots, badges, code signing, or a license.

- [ ] **Step 2: Validate README claims against evidence**

Check every feature, shortcut, model, privacy, and install claim against current code/configuration. Expected: README is useful to an end user in under two minutes and delegates lengthy procedures to the guides.

### Task 3: Create the Windows setup guide

**Files:**
- Create: `docs/SETUP.md`
- Reference: `package.json`
- Reference: `scripts/prepare-python-runtime.ps1`
- Reference: `src-tauri/python-runtime/runtime-manifest.json`
- Reference: `src-tauri/python-runtime/runtime-lock.json`

- [ ] **Step 1: Document packaged installation**

Cover supported OS, hardware/storage/network prerequisites verified by the manifests, downloading a future GitHub release without inventing a current asset name, Windows warnings, first launch/tray behavior, initial model installation, CPU/CUDA selection, microphone permission, and a short success check.

- [ ] **Step 2: Document source setup**

List exact Windows prerequisites and PowerShell commands for clone, `npm ci`, managed Python preparation, `npm run tauri:dev:managed-python`, frontend checks, Rust tests, Python tests, and `npm run tauri:build`. Explain which commands download large runtime/model artifacts and where failures are diagnosed.

- [ ] **Step 3: Verify every command**

Cross-check command spelling and environment variables against `package.json` and scripts. Expected: no Unix-only shell syntax and no undocumented prerequisite presented as optional.

### Task 4: Create the non-technical user guide

**Files:**
- Create: `docs/USER_GUIDE.md`
- Reference: `src/lib/components/Settings.svelte`
- Reference: `src/lib/stores/modelManager.ts`
- Reference: `src-tauri/src/runtime_profile/manifest.runtime-profiles.json`
- Reference: `src-tauri/src/transcribe.rs`

- [ ] **Step 1: Verify the first-use workflow and UI language**

Trace the current first launch, environment preparation, tray behavior, model download/activation states, default shortcut, transcription completion, clipboard write, and automatic insertion behavior. Record the exact labels and distinguish visible current behavior from intended future release behavior.

- [ ] **Step 2: Write the user walkthrough**

Create `docs/USER_GUIDE.md` with these sections in order: download from GitHub Releases; install on Windows; open Aywa and find its tray icon; wait for first-launch preparation; choose and download a model; activate it and confirm readiness; start/stop transcription; find and paste clipboard text; reopen Settings; exit; next help steps.

Keep build tools, terminals, runtime package names, and contributor commands out of the main flow. Use precise button/status names and short numbered procedures.

- [ ] **Step 3: Explain CPU and NVIDIA CUDA modes**

Explain in ordinary language that CPU is the compatible fallback and CUDA can substantially improve speed on a supported NVIDIA GPU, particularly with larger models. Recommend installing a current NVIDIA driver, tell users to choose only modes Aywa marks available, avoid promising support for every NVIDIA device, and link CUDA detection/activation failures to `TROUBLESHOOTING.md`.

- [ ] **Step 4: Verify user-facing claims**

Cross-check installation, tray, timing, model, shortcut, clipboard, paste, and acceleration claims against source and manifests. Expected: a first-time user can complete the workflow without reading `SETUP.md` or understanding the technology stack.

### Task 5: Create troubleshooting and support guides

**Files:**
- Create: `docs/TROUBLESHOOTING.md`
- Create: `docs/SUPPORT.md`
- Reference: `src/lib/components/Settings.svelte`
- Reference: `src-tauri/src/diagnostics.rs`

- [ ] **Step 1: Write symptom-based troubleshooting**

Use headings phrased as user-observable problems: app appears not to start; hotkey does nothing; microphone cannot record; transcription fails; text is not inserted; model install/download fails; managed Python fails; CUDA is unavailable; app is slow; log cannot be opened. For each, provide likely cause, safe recovery, and evidence to collect.

- [ ] **Step 2: Write the support workflow**

Document the exact current Debug labels and actions. Explain how to copy diagnosis, open/select `aywa-debug.log`, reproduce once, review/redact sensitive values, create an issue at `https://github.com/firasjaber/stt/issues/new/choose`, paste diagnosis in a code block, and drag the log file into the issue editor.

- [ ] **Step 3: Add privacy guidance**

Explicitly warn users to inspect logs for Windows usernames, local paths, command output, model/runtime locations, and any transcribed content observed by the audit. Never instruct users to publish a log without reviewing it.

### Task 6: Add the GitHub bug-report form

**Files:**
- Create: `.github/ISSUE_TEMPLATE/bug_report.yml`

- [ ] **Step 1: Create a valid issue form**

Set `name`, `description`, `title`, and `labels`, then add required fields for confirmation/search, Aywa version or commit, Windows version, installation method, reproduction steps, expected behavior, actual behavior, and diagnosis. Add an optional log-upload Markdown area and additional context field.

- [ ] **Step 2: Add the privacy acknowledgement**

Require acknowledgement that the reporter reviewed diagnosis/log content and removed sensitive information. Keep the log optional so inability to open it does not block reporting.

- [ ] **Step 3: Validate YAML structure**

Parse the file with an available YAML parser or manually verify it against GitHub issue-form schema when no parser is installed. Expected: unique IDs, valid input types, and `validations.required` only on actionable fields.

### Task 7: Cross-document verification and handoff

**Files:**
- Verify: `README.md`
- Verify: `docs/PUBLIC_RELEASE_AUDIT.md`
- Verify: `docs/SETUP.md`
- Verify: `docs/USER_GUIDE.md`
- Verify: `docs/TROUBLESHOOTING.md`
- Verify: `docs/SUPPORT.md`
- Verify: `.github/ISSUE_TEMPLATE/bug_report.yml`

- [ ] **Step 1: Check links, branding, scope, and placeholders**

Run:

```powershell
rg -n 'TBD|TODO|FIXME|sv create|ok-debug|\bOK\b|macOS|Linux' README.md docs/SETUP.md docs/USER_GUIDE.md docs/TROUBLESHOOTING.md docs/SUPPORT.md .github/ISSUE_TEMPLATE/bug_report.yml
git diff --check
```

Expected: operating-system references only explain the Windows-only boundary; no scaffold text, stale branding, or unresolved placeholders remain.

- [ ] **Step 2: Check repository-relative Markdown links**

Extract local Markdown link targets and confirm each target exists. Open direct GitHub links for syntactic review if network access is unavailable.

- [ ] **Step 3: Re-run proportional verification**

Run `npm.cmd run check`, `cargo test --manifest-path src-tauri/Cargo.toml`, and `python -m pytest src-tauri/python/tests -v`. Expected: documentation-only changes introduce no application regressions; report pre-existing failures separately.

- [ ] **Step 4: Review the final diff**

Run:

```powershell
git status --short
git diff -- README.md docs/PUBLIC_RELEASE_AUDIT.md docs/SETUP.md docs/USER_GUIDE.md docs/TROUBLESHOOTING.md docs/SUPPORT.md .github/ISSUE_TEMPLATE/bug_report.yml
```

Expected: only the seven intended deliverables appear in the implementation diff, alongside the user's untouched pre-existing changes.

- [ ] **Step 5: Present the release decision**

Summarize created files, test results, and the highest-priority blockers. Clearly separate “must fix before public” from improvements that can follow after publication.
