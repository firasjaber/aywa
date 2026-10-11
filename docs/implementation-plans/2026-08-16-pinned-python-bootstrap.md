# Pinned Python Bootstrap Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Pin `get-pip.py` to immutable upstream bytes and prevent the mutable URL from returning.

**Architecture:** The manifest uses the `pypa/get-pip` full commit `f6f644156f23dfe9acc06e7b9ca75eee311f2e37`; runtime preparation refreshes its matching SHA-256 lock. The release policy rejects non-commit-pinned bootstrap URLs.

**Tech Stack:** PowerShell, Node.js release-policy tests, JSON manifests.

---

### Task 1: Pin and verify the bootstrap source

**Files:**
- Modify: `src-tauri/python-runtime/runtime-manifest.json`
- Modify: `src-tauri/python-runtime/runtime-lock.json`
- Modify: `scripts/verify-release-policy.mjs`
- Modify: `scripts/verify-release-policy.test.mjs`

- [ ] **Step 1: Add the failing policy assertion**

Add a test that a `getPip.url` equal to `https://bootstrap.pypa.io/get-pip.py` makes the verifier fail, while `https://raw.githubusercontent.com/pypa/get-pip/f6f644156f23dfe9acc06e7b9ca75eee311f2e37/public/get-pip.py` passes.

- [ ] **Step 2: Enforce the raw GitHub commit URL**

Require the URL to match:

```js
/^https:\/\/raw\.githubusercontent\.com\/pypa\/get-pip\/[0-9a-f]{40}\/public\/get-pip\.py$/
```

- [ ] **Step 3: Pin and refresh lock**

Set the manifest URL to the passing URL, delete only `src-tauri/resources/python/get-pip.py`, and run `npm run prepare:python-runtime:lock` to calculate the lock from the pinned bytes.

- [ ] **Step 4: Validate and commit**

Run `node scripts/verify-release-policy.test.mjs` and `npm run prepare:python-runtime`; commit the manifest, lock, and policy tests.
