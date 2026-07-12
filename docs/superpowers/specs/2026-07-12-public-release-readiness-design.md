# Aywa Public Release Readiness Design

## Purpose

Prepare Aywa for a credible first public release by auditing the current repository and replacing the scaffold documentation with a Windows-focused documentation and support package. This work reports application-code risks and code smells but does not refactor unrelated product code.

## Product and audience

Aywa is a Windows-only desktop voice-typing application built with Tauri, Svelte, Rust, and a managed Python transcription runtime. The public documentation serves two distinct audiences:

- End users installing a packaged release and diagnosing common problems.
- Contributors building and running Aywa from source on Windows.

The documentation must not imply macOS or Linux support. It should explain local processing and privacy only to the extent that the implementation supports those claims.

## Deliverables

### Root README

Replace the Svelte scaffold `README.md` with a concise public landing page containing:

- Aywa's purpose, Windows-only status, and current maturity.
- Its main voice-typing workflow and user-visible features.
- A clear statement about local transcription, model downloads, and any network activity verified by the audit.
- A short release-installation path and a separate source-build entry point.
- Links to setup, troubleshooting, support, licensing, and issue reporting.
- A compact architecture and technology overview for contributors.
- No invented release links, badges, screenshots, or compatibility claims.

### Setup guide

Create `docs/SETUP.md` with two clearly separated flows:

1. Installing and starting a packaged Windows release, including prerequisites, model setup, initial verification, and Windows security prompts where applicable.
2. Building from source with the exact Node.js, Rust, Microsoft build tools, Python-runtime preparation, development, checking, testing, and packaging commands supported by the repository.

CPU and NVIDIA/CUDA behavior must be described from the implementation and runtime manifests rather than assumed. Unknown release-distribution details should be identified honestly instead of filled with placeholders.

### Troubleshooting guide

Create `docs/TROUBLESHOOTING.md` organized by observable symptom. Cover startup and tray behavior, microphone capture, hotkey registration, text insertion and clipboard behavior, model downloads, managed Python provisioning, CPU/CUDA selection, and diagnostic-log access. Each entry should give likely causes, safe recovery steps, and the evidence users should collect if recovery fails.

### Support and issue-submission guide

Create `docs/SUPPORT.md` describing:

- What information makes a report actionable.
- How to reproduce once before collecting evidence.
- How to open the Debug menu, copy the diagnosis, and locate the log file.
- How to paste diagnosis text into a GitHub issue and attach the log file.
- What potentially sensitive content to inspect and redact, including usernames, filesystem paths, command output, model paths, and dictated text if present.
- A direct link to the repository's issue tracker.

Create `.github/ISSUE_TEMPLATE/bug_report.yml` as a structured GitHub issue form. Require Windows version, Aywa version or commit, installation method, reproduction steps, expected and actual behavior, and diagnosis text. Request the debug log as an attachment but allow submission when it is unavailable. Include a privacy/redaction acknowledgement and avoid asking users to paste a large log inline.

### Public release audit

Create `docs/PUBLIC_RELEASE_AUDIT.md` as a durable, evidence-based review. Findings must be prioritized as:

- **Blocker**: unsafe or legally/release-critical; resolve before making the repository public.
- **High**: likely to harm users, leak sensitive data, break installation, or seriously undermine trust.
- **Medium**: maintainability, reliability, testing, or contributor-experience weakness worth fixing soon.
- **Low**: polish, consistency, or minor code-smell concern.

For every finding include the affected file and location, evidence, public impact, and a concrete recommendation. Cover at least:

- Secrets, personal data, logs, generated artifacts, and repository hygiene.
- License, authorship, repository metadata, version consistency, and third-party redistribution obligations.
- Tauri permissions and security configuration.
- Diagnostic logging and privacy characteristics.
- Download integrity, archive handling, subprocess execution, and filesystem safety.
- Error handling, concurrency, state management, process lifecycle, and resource cleanup.
- Rust, Python, TypeScript, and Svelte structure and code smells.
- Dependency pinning, reproducible builds, release packaging, and Windows-only consistency.
- Automated tests, CI, formatting, linting, and contribution workflow.
- First-impression issues such as scaffold text, stale names, committed logs, and undocumented behavior.

The audit distinguishes confirmed defects from risks that need verification. It does not modify application behavior as part of this documentation pass.

## Review method

Inspect all first-party source, configuration, scripts, manifests, tests, documentation, tracked-file state, and recent repository history. Use focused searches for credentials, absolute personal paths, panic-prone constructs, ignored errors, unfinished markers, unsafe code, network endpoints, command invocation, logging, and stale branding. Run the repository's existing frontend, Rust, and Python checks/tests where the local environment permits. Record tool or environment limitations in the audit.

Existing uncommitted changes are treated as the current intended product state and must be preserved. Documentation edits should not overwrite unrelated user changes.

## Documentation style

Use direct, calm language suitable for a small open-source project. Lead with the shortest successful path, keep advanced diagnostics in linked guides, use PowerShell-compatible commands, and avoid promotional claims that the code cannot substantiate. Cross-link the four documents so users can move from installation to recovery to issue submission without searching the repository.

## Validation

- Verify all relative Markdown links and GitHub URLs.
- Verify referenced menu labels, buttons, diagnosis fields, and log behavior against the current UI and Rust commands.
- Verify documented commands against `package.json`, Cargo configuration, Python scripts, and existing tests.
- Run frontend checks, Rust tests, and Python tests when possible.
- Search new documents for placeholders, stale `OK` branding, unsupported operating-system claims, and contradictions.
- Review the final diff to ensure only the intended documentation and issue-template files changed.

## Out of scope

- Refactoring application code solely to address audit findings.
- Adding macOS or Linux support.
- Creating release binaries, publishing a GitHub release, or changing repository visibility.
- Inventing screenshots, release URLs, signing instructions, or support guarantees that do not exist.
- Selecting a software license on the owner's behalf; absence of a license is reported as a release blocker for the owner to resolve.
