# Signed Windows Releases Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Ship verified Windows-only Aywa releases from `v0.1.0` tags with required Authenticode signing and published SHA-256 checksums.

**Architecture:** GitHub Actions uses `windows-2022` for CI and tagged releases. A Node policy verifier makes version, Windows target, and required policy files testable; the release workflow runs the test suite, signs and verifies the NSIS installer, checks a silent install on the clean runner, then creates the GitHub Release.

**Tech Stack:** GitHub Actions, PowerShell, SignTool, Node.js test runner, npm, Rust/Cargo, pytest, Tauri 2.

---

## File map

- Create `.github/workflows/ci.yml`: Windows-only pull-request and main-push validation.
- Create `.github/workflows/release.yml`: fail-closed `v*` signed release pipeline.
- Create `rust-toolchain.toml`: Rust 1.95.0 compiler declaration for local and CI use.
- Create `SECURITY.md`: vulnerability-reporting policy.
- Create `docs/RELEASE_CHECKLIST.md`: certificate setup, release, verification, rollback, and tag guidance.
- Modify `package.json`: set frontend package version to `0.1.0` and add deterministic release checks.
- Modify `package-lock.json`: synchronize the root package version to `0.1.0` with npm.
- Modify `src-tauri/Cargo.toml`: replace the placeholder author with `Firas Jaber`.
- Modify `src-tauri/tauri.conf.json`: set the Windows NSIS bundle target explicitly.
- Modify `scripts/verify-release-policy.mjs`: validate synchronized metadata, Windows scope, and mandatory release/security files.
- Modify `scripts/verify-release-policy.test.mjs`: cover the new policy requirements and release-tag behavior.

### Task 1: Make metadata and policy validation executable

**Files:**
- Modify: `package.json`
- Modify: `package-lock.json`
- Modify: `src-tauri/Cargo.toml:1-10`
- Modify: `src-tauri/tauri.conf.json:1-60`
- Modify: `scripts/verify-release-policy.mjs`
- Modify: `scripts/verify-release-policy.test.mjs`

- [ ] **Step 1: Write failing version/Windows policy tests**

Extend the temporary fixture to write a Tauri config and required policy files, then add assertions such as:

```js
assert.deepEqual(await verifyReleasePolicy(root, { tag: 'v0.1.0' }), []);
assert.deepEqual(await verifyReleasePolicy(root, { tag: 'v0.2.0' }), [
  'Release tag v0.2.0 must match version 0.1.0.'
]);
```

Add individual fixtures that prove failures for a frontend/Cargo/Tauri version mismatch, `targets: "all"`, absent `SECURITY.md`, and absent `docs/RELEASE_CHECKLIST.md`.

- [ ] **Step 2: Run the Node test before implementation**

Run:

```powershell
node --test scripts/verify-release-policy.test.mjs
```

Expected: FAIL because the current verifier accepts only license/notice metadata and does not accept a tag argument.

- [ ] **Step 3: Align checked-in release metadata**

Set the root package version to `0.1.0`, run `npm install --package-lock-only --ignore-scripts` to update only the lockfile root metadata, set Cargo authors to `Firas Jaber`, and replace the Tauri bundle target with:

```json
"targets": ["nsis"]
```

Keep `src-tauri/tauri.conf.json` as the checked-in canonical app-version source; all three manifests must contain the identical value.

- [ ] **Step 4: Implement policy checks**

Update `verifyReleasePolicy(root, { tag } = {})` to parse `package.json` and `src-tauri/tauri.conf.json`, read `SECURITY.md` and `docs/RELEASE_CHECKLIST.md`, and add failures for unequal versions, a Cargo placeholder author, a non-array/non-`nsis` target, missing policy documents, or a supplied `v<version>` tag that differs from the manifest version. Update CLI parsing to accept `--tag v0.1.0`.

- [ ] **Step 5: Re-run policy tests and lint the diff**

Run:

```powershell
node --test scripts/verify-release-policy.test.mjs
npm run verify:release-policy
git diff --check
```

Expected: all Node tests pass; the policy command reports no failures for the checkout.

- [ ] **Step 6: Commit the metadata/policy gate**

```powershell
git add package.json package-lock.json src-tauri/Cargo.toml src-tauri/tauri.conf.json scripts/verify-release-policy.mjs scripts/verify-release-policy.test.mjs
git commit -m "build: validate release metadata"
```

### Task 2: Add locked, Windows-only continuous integration

**Files:**
- Create: `rust-toolchain.toml`
- Create: `.github/workflows/ci.yml`

- [ ] **Step 1: Add the Rust toolchain declaration**

Create this exact file:

```toml
[toolchain]
channel = "1.95.0"
profile = "minimal"
components = ["rustfmt"]
targets = ["x86_64-pc-windows-msvc"]
```

- [ ] **Step 2: Add the Windows CI workflow**

Create a workflow triggered on `pull_request` and pushes to `master`, with `runs-on: windows-2022`, `permissions: { contents: read }`, and concurrency cancellation for non-default branch runs. Use `actions/checkout` at a fixed major version, `actions/setup-node` at a fixed major version with `node-version-file: package.json` only after adding `engines.node`, or an explicit Node 22 version. Run these commands in separate steps:

```powershell
npm ci
npm run verify:release-policy
npm run check
npm run build
cargo test --manifest-path src-tauri/Cargo.toml --locked
python -m pytest src-tauri/python/tests -v
```

Do not invoke managed Python runtime provisioning in CI: it downloads heavyweight runtime/model dependencies and is not needed by the focused Python protocol tests.

- [ ] **Step 3: Validate workflow syntax and references**

Run:

```powershell
rg -n "runs-on: windows-2022|npm ci|verify:release-policy|--locked|pytest" .github/workflows/ci.yml rust-toolchain.toml
git diff --check
```

Expected: every required CI gate and Windows runner appears once; there are no whitespace errors.

- [ ] **Step 4: Commit CI**

```powershell
git add rust-toolchain.toml .github/workflows/ci.yml package.json
git commit -m "ci: validate Windows builds"
```

### Task 3: Build a fail-closed signed release workflow

**Files:**
- Create: `.github/workflows/release.yml`

- [ ] **Step 1: Add tag/version and secret preflight**

Create a `push.tags: ["v*"]` workflow on `windows-2022` with `contents: write`. Derive the tag from `github.ref_name`, then execute:

```powershell
node scripts/verify-release-policy.mjs --tag "$env:RELEASE_TAG"
if ([string]::IsNullOrWhiteSpace($env:WINDOWS_CERTIFICATE_PFX_BASE64) -or [string]::IsNullOrWhiteSpace($env:WINDOWS_CERTIFICATE_PASSWORD)) {
  throw 'Windows signing secrets are required for public releases.'
}
```

Pass `github.ref_name` and the two GitHub secrets through that step's `env`. Never print secret values.

- [ ] **Step 2: Add build, signing, and signature verification**

After `npm ci` and the same CI test commands, build only the NSIS bundle with `npm run tauri -- build --bundles nsis`. Decode the PFX into `$env:RUNNER_TEMP\aywa-signing.pfx`, invoke the Windows SDK `signtool.exe` with SHA-256 digest and timestamping, then run `signtool verify /pa /v` on every produced `.exe`. Register a PowerShell `try`/`finally` block that removes the PFX before the step ends.

- [ ] **Step 3: Add artifacts, hashes, and clean-machine smoke test**

Locate exactly one NSIS installer under `src-tauri\target\release\bundle\nsis`, copy it to a `release-assets` directory, and generate `SHA256SUMS.txt` with:

```powershell
Get-ChildItem release-assets -File -Filter '*.exe' |
  Get-FileHash -Algorithm SHA256 |
  ForEach-Object { "{0} *{1}" -f $_.Hash.ToLowerInvariant(), $_.Name } |
  Set-Content -NoNewline release-assets\SHA256SUMS.txt
```

Silent-install the copied EXE using its NSIS `/S` switch, assert the expected installed `Aywa.exe` exists under `$env:LOCALAPPDATA\Aywa`, invoke the uninstaller silently if it exists, and fail when the executable is absent.

- [ ] **Step 4: Publish only verified artifacts**

Use `softprops/action-gh-release` at a pinned major version after all build/sign/smoke steps. Attach only `release-assets\*.exe` and `release-assets\SHA256SUMS.txt`, set the generated release notes flag, and use the tag name as the release name. Upload the same files as a workflow artifact with `actions/upload-artifact` to retain debugging evidence if publication fails.

- [ ] **Step 5: Inspect release gates statically**

Run:

```powershell
rg -n "v\*|windows-2022|WINDOWS_CERTIFICATE|signtool|verify /pa|SHA256SUMS|/S|action-gh-release" .github/workflows/release.yml
git diff --check
```

Expected: the workflow contains no alternate unsigned publication path and all pre-publication gates precede the GitHub Release action.

- [ ] **Step 6: Commit release automation**

```powershell
git add .github/workflows/release.yml
git commit -m "ci: publish signed Windows releases"
```

### Task 4: Document security, release operations, and tagging

**Files:**
- Create: `SECURITY.md`
- Create: `docs/RELEASE_CHECKLIST.md`
- Modify: `README.md`
- Modify: `docs/SETUP.md`

- [ ] **Step 1: Write the security policy**

Document supported versions (`0.1.x`), an email or GitHub private advisory reporting path without committing a personal address, a request not to disclose vulnerabilities publicly before a fix, acknowledgement/triage targets stated as goals rather than guarantees, and disclosure coordination.

- [ ] **Step 2: Write the release checklist**

Explain the two GitHub Actions secrets precisely, including a PowerShell base64 command that reads a local PFX without committing it. Include required preflight checks, workflow behavior, checksum verification, Authenticode verification, certificate revocation/compromise actions, and this exact first-release process:

```powershell
git status
git tag -a v0.1.0 -m "Aywa 0.1.0"
git push origin v0.1.0
```

State that tags are immutable release identifiers and correct a tag mistake by creating a new version rather than force-moving a public tag.

- [ ] **Step 3: Update user-facing Windows/release documentation**

Change README and setup wording from "signing has not been finalized" to explain that official installers come from GitHub Releases, carry a verified publisher signature, and have a `SHA256SUMS.txt` companion file. State Windows-only support and link `SECURITY.md` and the release checklist.

- [ ] **Step 4: Verify documentation references**

Run:

```powershell
rg -n "Windows only|SHA256SUMS|code signing|v0.1.0|SECURITY.md|RELEASE_CHECKLIST" README.md docs/SETUP.md docs/RELEASE_CHECKLIST.md SECURITY.md
git diff --check
```

Expected: documentation describes only signed Windows NSIS releases and has working repository-relative links.

- [ ] **Step 5: Commit release documentation**

```powershell
git add SECURITY.md docs/RELEASE_CHECKLIST.md README.md docs/SETUP.md
git commit -m "docs: document signed release process"
```

### Task 5: Run final local verification and handoff

**Files:**
- Verify: all files above

- [ ] **Step 1: Run release policy and its tests**

Run:

```powershell
node --test scripts/verify-release-policy.test.mjs
npm run verify:release-policy
node scripts/verify-release-policy.mjs --tag v0.1.0
```

Expected: all pass; the explicit `v0.1.0` tag matches all manifests.

- [ ] **Step 2: Run application checks**

Run:

```powershell
npm run check
npm run build
cargo test --manifest-path src-tauri/Cargo.toml --locked
python -m pytest src-tauri/python/tests -v
```

Expected: all checks pass. Report any environmental limitation distinctly from a product failure.

- [ ] **Step 3: Review final scope**

Run:

```powershell
git status --short
git diff --check HEAD~4..HEAD
```

Expected: the implementation includes only release metadata, policies, scripts, workflows, toolchain declaration, and release documentation.

- [ ] **Step 4: Configure secrets and publish the first release**

In GitHub repository Settings, add `WINDOWS_CERTIFICATE_PFX_BASE64` and `WINDOWS_CERTIFICATE_PASSWORD` as Actions secrets. After the implementation commits are pushed to GitHub and the CI workflow is green, run the documented `git tag -a v0.1.0 -m "Aywa 0.1.0"` and `git push origin v0.1.0` commands. Confirm the release job shows successful signing, checksum generation, smoke install, and GitHub Release publication.
