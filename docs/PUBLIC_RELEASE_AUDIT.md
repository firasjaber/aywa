# Aywa public-release audit

Audit date: 2026-07-12. Scope: the current dirty worktree, treated as the intended product state. This review did not refactor application code.

## Release recommendation

**Do not make the repository public as a finished open-source release yet.** The codebase has a healthy passing baseline, but it lacks a license and currently retains microphone recordings indefinitely. Resolve the blockers and high-priority privacy/support items before inviting general users.

## Verification results

- `npm run check`: passed with 0 errors and 0 warnings.
- `npm run build`: passed.
- `cargo test --manifest-path src-tauri/Cargo.toml`: 64 passed.
- `python -m pytest src-tauri/python/tests -v`: 7 passed.
- Secret-pattern review found no committed application credentials. Large vendored/generated files still require provenance review.

## Blockers

### Blocker: no software license

- **Evidence:** `src-tauri/Cargo.toml:6` has an empty license, and the repository has no `LICENSE` file.
- **Impact:** Public visibility alone does not grant permission to use, modify, or distribute the source. Contributors and packagers cannot safely participate.
- **Recommendation:** Select an OSI-approved license, add the full license text, populate Cargo/package metadata, and verify compatibility with Tauri, Python, PyTorch, NeMo, NVIDIA model, and bundled Python/get-pip redistribution terms.

### Blocker: captured microphone recordings are retained indefinitely

- **Evidence:** `src-tauri/src/lib.rs:60` stores WAV files under `runtime/recordings`; `stop_recording` transcribes them but never deletes them. No recording-retention control or cleanup path exists.
- **Impact:** Private speech remains on disk without clear user consent, retention limits, or UI controls. This is a major trust and privacy problem for a voice app.
- **Recommendation:** Delete each WAV after transcription on both success and failure using a guaranteed cleanup guard. Document any intentional retention and add startup cleanup for abandoned files plus tests.

## High priority

### High: diagnostics are unbounded and expose local machine details

- **Evidence:** `src-tauri/src/diagnostics.rs:26` opens `aywa-debug.log` in append mode with no rotation; lines 122-172 log full commands/output. `transcribe.rs:626` and related calls log Python, model, and recording paths.
- **Impact:** Logs grow indefinitely and disclose usernames, filesystem layout, model paths, recording names, GPU details, and subprocess output when shared.
- **Recommendation:** Add size-based rotation and retention, minimize/redact paths, document contents, and provide a generated issue report with an explicit privacy preview.

### High: the requested issue-report action is visibly disabled

- **Evidence:** `src/lib/components/Settings.svelte:343` renders **Copy issue report** disabled with `title="Coming later"`.
- **Impact:** Users are told diagnostics exist for support but cannot collect them reliably; manual copying is error-prone.
- **Recommendation:** Implement a deterministic, redacted diagnosis report and clipboard action before directing general users to submit it. Until then, keep support docs honest about manual copying.

### High: public distribution and signing are not automated or documented

- **Evidence:** no `.github/workflows`, release workflow, signing configuration, checksum publication process, `SECURITY.md`, or release checklist exists. `tauri.conf.json` targets all platforms although product support is Windows-only.
- **Impact:** Releases are difficult to reproduce and verify, unsigned installers damage trust, and accidental non-Windows artifacts may imply unsupported compatibility.
- **Recommendation:** Add a Windows-only CI/release workflow with locked toolchains, tests, artifact hashes, code signing, clean-machine smoke tests, and a security policy.

### High: runtime/model dependency trust needs a distribution review

- **Evidence:** embedded Python and `get-pip.py` are checksum locked, and model downloads are checked against Hugging Face LFS hashes. Runtime preparation still installs packages from live indexes, while model metadata is resolved from mutable `main`; `src-tauri/resources/python/get-pip.py` is a large vendored executable script.
- **Impact:** Builds and first-run environments can change or fail as upstream indexes change, and redistributed third-party components carry license/provenance obligations.
- **Recommendation:** Produce an SBOM and third-party notices, pin transitive Python dependencies with hashes, record model revisions, document network endpoints, and review licenses before distribution.

## Medium priority

### Medium: CSP is disabled

- **Evidence:** `src-tauri/tauri.conf.json:40` sets `"csp": null`.
- **Impact:** A future frontend injection or remote-content change would lack an important defense-in-depth boundary in the privileged desktop webview.
- **Recommendation:** Define the narrowest Tauri CSP compatible with local assets and required IPC; keep remote network activity in Rust rather than the webview.

### Medium: release metadata is incomplete and inconsistent

- **Evidence:** frontend version is `0.0.1`; Cargo and Tauri versions are `0.1.0`; Cargo authors is `"you"`; license/repository are blank; remote repository name remains `stt` while the app is Aywa.
- **Impact:** About dialogs, packages, crash reports, and contributor expectations can disagree; placeholder authorship looks unfinished.
- **Recommendation:** Choose one version source, synchronize manifests in release automation, fill project metadata, and decide whether to rename the GitHub repository.

### Medium: several large modules combine unrelated responsibilities

- **Evidence:** `model_manager/service.rs` is about 1,043 lines, `transcribe.rs` 1,003, `Settings.svelte` 891, and `runtime_profile/service.rs` 517.
- **Impact:** State transitions, filesystem work, process control, UI rendering, and copy generation are difficult to review and test independently.
- **Recommendation:** Split by bounded responsibility during feature work: orchestration versus persistence/events, worker process versus provisioning/protocol, and settings status/models/diagnostics components.

### Medium: unsafe thread traits lack a documented invariant

- **Evidence:** `src-tauri/src/hotkey.rs:14` manually implements `Send` and `Sync` for `HotkeyHandle`.
- **Impact:** If upstream handle types are thread-affine, moving them through Tauri managed state can cause undefined or platform-specific behavior.
- **Recommendation:** Document and test the safety invariant, link the upstream guarantee, or keep creation/use/drop on one owning thread and avoid unsafe implementations.

### Medium: diagnostics truncation can panic on Unicode boundaries

- **Evidence:** `src-tauri/src/diagnostics.rs:262` truncates with `&value[..MAX_LEN]` after checking byte length.
- **Impact:** Non-ASCII subprocess output whose 800th byte splits a character can panic precisely while reporting another failure.
- **Recommendation:** Truncate at a valid `char_indices` boundary and add multibyte tests.

### Medium: downloads have weak resource bounds and cleanup

- **Evidence:** `model_manager/download.rs` streams without an explicit client timeout or maximum byte check, logs `expected_size` but does not enforce it, and leaves unique temporary files on interrupted/error paths.
- **Impact:** Stalled/oversized responses and repeated failures can consume time and disk space despite the final checksum check.
- **Recommendation:** Configure connect/total timeouts, enforce expected size while streaming, use a cleanup guard, and remove stale temp files at startup.

### Medium: support/help UI contains dead links

- **Evidence:** `Settings.svelte:193` and `:277` use fragment links whose click handlers only call `preventDefault()` for CUDA/setup help.
- **Impact:** The polished UI promises help but performs no action, creating a poor first impression during failure recovery.
- **Recommendation:** Open the relevant bundled/web documentation or remove the links until navigation exists.

### Medium: automated coverage omits key end-to-end risks

- **Evidence:** Rust/Python unit tests are substantial, but there is no CI, frontend test runner, installer test, clean-machine first-run test, real microphone/hotkey/clipboard integration test, or failure-path retention test.
- **Impact:** Packaging, Windows permissions, input simulation, large downloads, and privacy cleanup can regress while unit tests remain green.
- **Recommendation:** Add Windows CI, component tests for status/actions, mocked download/provision integration tests, and a release smoke-test checklist.

## Low priority

### Low: repository contains development debris and stale branding

- **Evidence:** tracked `run_dev_output.log`, scaffold README before this documentation pass, `src/lib/assets/favicon.svg` remains the Svelte logo, personal absolute command in `docs/model-manager-smoke-checks.md:26`, and historical docs prominently use the former OK name.
- **Impact:** Visitors may assume the project is an unclean prototype or accidentally learn a maintainer’s local username/path.
- **Recommendation:** Remove the tracked log, replace/remove the stale favicon, make commands portable, and archive or label historical design documents rather than rewriting history silently.

### Low: ignored errors obscure operational failures

- **Evidence:** tray callbacks, overlay reset threads, hotkey events, and log writes frequently use `let _ = ...`.
- **Impact:** Failures disappear or surface only indirectly, making support harder.
- **Recommendation:** Log actionable failures with context and reserve ignored results for explicitly harmless best-effort cleanup.

### Low: Windows-only scope is not consistently encoded

- **Evidence:** `tauri.conf.json` uses `"targets": "all"`, while PowerShell/npm scripts and diagnostics log opening are Windows-specific.
- **Impact:** Contributors may spend time attempting unsupported builds or publish misleading artifacts.
- **Recommendation:** Configure/document Windows targets explicitly and gate Windows-only modules at compile time.

## Strengths to preserve

- Model files are streamed and SHA-256 verified before promotion.
- Embedded Python resources are locked by checksum and ZIP extraction uses `enclosed_name` against traversal.
- Rust and Python protocols have focused unit tests; all current checks pass.
- The app falls back to CPU when compatible CUDA execution is unavailable.
- Network operations use HTTPS and the webview has minimal declared Tauri capabilities.
- Transcript text is not intentionally written to the diagnostic log.

## Pre-publication checklist

- [ ] Add and review a project license and third-party notices.
- [ ] Delete recordings reliably and clean abandoned recordings.
- [ ] Add log rotation/redaction and implement **Copy issue report**.
- [ ] Remove `run_dev_output.log`, personal paths, and stale scaffold assets.
- [ ] Replace dead help links with working documentation navigation.
- [ ] Align project versions, authorship, repository metadata, and Windows bundle targets.
- [ ] Add Windows CI/release automation, signing, checksums, and clean-machine tests.
- [ ] Add `SECURITY.md` and basic contribution guidance.
- [ ] Review Python/model dependency pinning, provenance, redistribution rights, and SBOM output.
