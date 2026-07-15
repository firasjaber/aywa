# Unsigned Windows Releases Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Publish explicitly unsigned Windows installers from version tags while retaining tests, SHA-256 checksums, and clean-runner install validation.

**Architecture:** The existing Windows release workflow remains tag-triggered and validates metadata before building the NSIS installer. Remove all certificate and signature operations, then publish only the unsigned installer and its checksum after the smoke install succeeds; documentation explains checksum verification does not prove publisher identity.

**Tech Stack:** GitHub Actions, PowerShell, Node.js test runner, npm, Rust/Cargo, pytest, Tauri 2, Markdown.

---

## File map

- Modify `.github/workflows/release.yml`: remove certificate secrets and signing/verification steps; preserve build, hashing, smoke install, and release creation.
- Modify `docs/RELEASE_CHECKLIST.md`: remove certificate setup/signature verification and document unsigned release/tag process.
- Modify `README.md`, `docs/SETUP.md`, and `docs/USER_GUIDE.md`: accurately explain unsigned installers, SmartScreen/unknown publisher, official source, and checksums.
- Modify `scripts/verify-release-policy.mjs` and its test: require unsigned-release documentation rather than signing operations.

### Task 1: Update the release workflow and policy tests

**Files:**
- Modify: `.github/workflows/release.yml`
- Modify: `scripts/verify-release-policy.mjs`
- Modify: `scripts/verify-release-policy.test.mjs`

- [ ] **Step 1: Write failing policy assertions**

Add a fixture assertion that accepts the required release documentation when it
contains `unsigned` and `SHA256SUMS.txt`. Add a failing fixture whose release
checklist omits either phrase and assert the exact policy failure message.

- [ ] **Step 2: Run policy tests before implementation**

Run:

```powershell
node --test scripts/verify-release-policy.test.mjs
```

Expected: FAIL because the current policy does not inspect unsigned-release
documentation.

- [ ] **Step 3: Remove signing operations**

In `release.yml`, rename the workflow to `Unsigned Windows Release`; remove the
two certificate environment variables, secret preflight, PFX decoding,
SignTool lookup/invocation, and signature verification. Keep the tag policy
check, test/build steps, NSIS installer lookup, checksum generation, silent
install, artifact upload, and GitHub Release action. Rename the smoke-test step
to `Create checksums and smoke-install the unsigned installer`.

- [ ] **Step 4: Make unsigned documentation policy-enforced**

Read `docs/RELEASE_CHECKLIST.md` in `verifyReleasePolicy`. Add failures unless
it contains both `unsigned` and `SHA256SUMS.txt`. Update fixtures to provide a
minimal valid unsigned checklist and assert the two missing-phrase failures.

- [ ] **Step 5: Re-run the policy suite**

Run:

```powershell
node --test scripts/verify-release-policy.test.mjs
npm.cmd run verify:release-policy
node scripts/verify-release-policy.mjs --tag v0.1.0
```

Expected: all commands pass without certificate secrets.

- [ ] **Step 6: Commit workflow and policy change**

```powershell
git add .github/workflows/release.yml scripts/verify-release-policy.mjs scripts/verify-release-policy.test.mjs
git commit -m "ci: publish unsigned Windows releases"
```

### Task 2: Rewrite release and user documentation

**Files:**
- Modify: `docs/RELEASE_CHECKLIST.md`
- Modify: `README.md`
- Modify: `docs/SETUP.md`
- Modify: `docs/USER_GUIDE.md`

- [ ] **Step 1: Rewrite the release checklist**

Remove certificate/PFX/secret instructions, Authenticode checks, and certificate
incident handling. State that releases are unsigned, retain the `v0.1.0` tag
commands, document the automatic test/build/hash/smoke-install/release flow,
and show `Get-FileHash` comparison against `SHA256SUMS.txt`. State that a hash
does not identify the publisher.

- [ ] **Step 2: Update public installation guidance**

In README and setup guide, replace signed/publisher claims with: official
installers are unsigned; download only from Aywa GitHub Releases; compare the
installer SHA-256 value against `SHA256SUMS.txt`; Windows may show an unknown
publisher or SmartScreen warning; checksums verify file integrity only.

- [ ] **Step 3: Update the non-technical user guide**

Correct the releases URL to `https://github.com/firasjaber/aywa/releases`.
Before the installer step, explain in plain language that the installer is
unsigned and Windows may warn; users should continue only when the GitHub
release source and checksum match their expectation.

- [ ] **Step 4: Verify public claims**

Run:

```powershell
rg -n -i "signed|authenticode|certificate|pfx|signtool|unknown publisher|smartscreen|sha256sums|unsigned" README.md docs/RELEASE_CHECKLIST.md docs/SETUP.md docs/USER_GUIDE.md .github/workflows/release.yml
git diff --check
```

Expected: active release documentation contains no claim that Aywa installers
are signed; all four user-facing documents disclose the unsigned status.

- [ ] **Step 5: Commit documentation**

```powershell
git add README.md docs/RELEASE_CHECKLIST.md docs/SETUP.md docs/USER_GUIDE.md
git commit -m "docs: disclose unsigned Windows releases"
```

### Task 3: Final verification and handoff

**Files:**
- Verify: `.github/workflows/release.yml`, `scripts/verify-release-policy.mjs`, `scripts/verify-release-policy.test.mjs`, `README.md`, `docs/RELEASE_CHECKLIST.md`, `docs/SETUP.md`, `docs/USER_GUIDE.md`

- [ ] **Step 1: Confirm signing material is absent**

Run:

```powershell
rg -n -i "WINDOWS_CERTIFICATE|signTool|signtool|\.pfx|authenticode" .github/workflows scripts README.md docs/RELEASE_CHECKLIST.md docs/SETUP.md docs/USER_GUIDE.md
```

Expected: no output.

- [ ] **Step 2: Run proportional checks**

Run:

```powershell
npm.cmd run check
npm.cmd run build
cargo test --manifest-path src-tauri/Cargo.toml --locked
python -m pytest src-tauri/python/tests -v
```

Expected: tests and builds pass; report any unrelated environment limitation.

- [ ] **Step 3: Review final changes**

Run:

```powershell
git status --short
git diff --check
```

Expected: only unsigned-release workflow, policy, documentation, and plan
changes are present.
