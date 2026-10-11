# Tag-Only Release Validation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Fix tagged Aywa releases by preparing required Tauri resources and installing pytest before release validation, while running no CI on branches or pull requests.

**Architecture:** Remove the standalone branch CI workflow. The tag-only release workflow installs a pinned test dependency and prepares checksum-locked Python bundle resources before its Rust test, then retains the existing build, checksum, smoke-install, and release stages.

**Tech Stack:** GitHub Actions, PowerShell, pip, pytest, npm, Rust/Cargo, Tauri 2.

---

### Task 1: Define and use the test dependency

**Files:**
- Create: `src-tauri/python/requirements-test.txt`
- Modify: `.github/workflows/release.yml`

- [ ] **Step 1: Add a deterministic pytest requirement**

Create `src-tauri/python/requirements-test.txt`:

```text
pytest==8.3.5
```

- [ ] **Step 2: Provision release test prerequisites**

After `npm ci`, add separate workflow steps:

```yaml
- name: Install Python test dependency
  run: python -m pip install --disable-pip-version-check --requirement src-tauri/python/requirements-test.txt
- name: Prepare embedded Python resources
  run: npm run prepare:python-runtime
```

Place them before `Run checks and tests`; remove the former duplicate resource-preparation step later in the workflow.

- [ ] **Step 3: Run the known failure sequence locally**

Run:

```powershell
python -m pip install --disable-pip-version-check --requirement src-tauri/python/requirements-test.txt
npm.cmd run prepare:python-runtime
cargo test --manifest-path src-tauri/Cargo.toml --locked
python -m pytest src-tauri/python/tests -v
```

Expected: Tauri finds `resources/python/*`; all Rust and Python tests pass.

### Task 2: Restrict automation to tags

**Files:**
- Delete: `.github/workflows/ci.yml`
- Modify: `docs/RELEASE_CHECKLIST.md`

- [ ] **Step 1: Remove branch/pull-request CI**

Delete `.github/workflows/ci.yml`. `release.yml` remains the only workflow and
uses `push.tags: ["v*"]`.

- [ ] **Step 2: Document tag-only validation**

Add a preflight reminder to `docs/RELEASE_CHECKLIST.md` that branch and pull
request automation is intentionally disabled; local checks must pass before
pushing a release tag because GitHub performs heavyweight validation only after
the tag is pushed.

- [ ] **Step 3: Validate workflow scope**

Run:

```powershell
Test-Path .github/workflows/ci.yml
rg -n "tags: \[\"v\*\"\]|prepare:python-runtime|requirements-test" .github/workflows/release.yml
```

Expected: `Test-Path` prints `False`; the release workflow contains all three
required tag-only preparation markers.

### Task 3: Final verification and commit

**Files:**
- Verify: `.github/workflows/release.yml`, `src-tauri/python/requirements-test.txt`, `docs/RELEASE_CHECKLIST.md`

- [ ] **Step 1: Run release policy and application checks**

Run:

```powershell
node --test scripts/verify-release-policy.test.mjs
npm.cmd run verify:release-policy
npm.cmd run check
npm.cmd run build
```

Expected: all checks pass.

- [ ] **Step 2: Review and commit**

Run:

```powershell
git diff --check
git add .github/workflows/ci.yml .github/workflows/release.yml src-tauri/python/requirements-test.txt docs/RELEASE_CHECKLIST.md docs/superpowers/plans/2026-07-15-tag-only-release-validation.md
git commit -m "ci: prepare tagged release validation"
```

Expected: only the tag-only validation fix and its documentation/plan are committed.
