# Aywa Licensing and Third-Party Distribution Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** License Aywa-owned work under MIT and establish a verifiable public policy for third-party release artifacts.

**Architecture:** Put the license decision in standard root metadata files, retain a plain Markdown inventory for distributed third-party material, and use a zero-dependency Node script to reject missing policy inputs. This detects policy drift but does not replace legal review or generate an incomplete SBOM.

**Tech Stack:** Markdown, JSON, Cargo TOML metadata, Node.js built-in test runner.

---

## File structure

- Create: `LICENSE` â€” canonical MIT License for Aywa-owned source and documentation.
- Create: `NOTICE` â€” Aywa attribution and link to third-party notices.
- Create: `THIRD_PARTY_NOTICES.md` â€” inventory, first-run download policy, endpoints, and release gate.
- Create: `scripts/verify-release-policy.mjs` â€” static release-policy verifier.
- Create: `scripts/verify-release-policy.test.mjs` â€” test coverage using a temporary fixture repository.
- Modify: `package.json` â€” SPDX license metadata and verifier command.
- Modify: `src-tauri/Cargo.toml` â€” SPDX license metadata and canonical repository URL.
- Modify: `README.md` â€” MIT license and third-party policy links; canonical repository links.

### Task 1: Add and test the release-policy verifier

**Files:**

- Create: `scripts/verify-release-policy.mjs`
- Create: `scripts/verify-release-policy.test.mjs`

- [ ] **Step 1: Write the failing test**

Create the test before the module using `node:test`, `mkdtemp`, and `writeFile`. Build a complete temporary fixture with `LICENSE`, `NOTICE`, `THIRD_PARTY_NOTICES.md`, `package.json`, and `Cargo.toml`; assert `verifyReleasePolicy(root)` returns `[]`. Create an incomplete fixture whose package license is `Apache-2.0` and notices only contain `# Incomplete`; assert the failures include every required phrase.

```js
test('accepts a complete MIT release policy', async () => {
  const root = await createCompleteFixture();
  assert.deepEqual(await verifyReleasePolicy(root), []);
});

test('reports incomplete release policy data', async () => {
  const root = await createIncompleteFixture();
  assert.deepEqual(await verifyReleasePolicy(root), [
    'package.json must declare license "MIT".',
    'THIRD_PARTY_NOTICES.md must mention "CPython embeddable distribution".',
    'THIRD_PARTY_NOTICES.md must mention "get-pip.py".',
    'THIRD_PARTY_NOTICES.md must mention "python.org".',
    'THIRD_PARTY_NOTICES.md must mention "bootstrap.pypa.io".',
    'THIRD_PARTY_NOTICES.md must mention "download.pytorch.org".',
    'THIRD_PARTY_NOTICES.md must mention "huggingface.co".',
    'THIRD_PARTY_NOTICES.md must mention "SBOM".'
  ]);
});
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `node --test scripts/verify-release-policy.test.mjs`

Expected: FAIL because the verifier module does not exist.

- [ ] **Step 3: Implement the minimum verifier**

Create `scripts/verify-release-policy.mjs`. It reads the five root files, parses `package.json`, checks the MIT license headings/metadata and Cargo repository URL, and appends one error per missing required phrase. Export `verifyReleasePolicy(root)`; when run as a CLI, print all errors with a `release-policy:` prefix and exit nonzero on failures.

```js
const requiredNoticePhrases = [
  'CPython embeddable distribution', 'get-pip.py', 'python.org',
  'bootstrap.pypa.io', 'download.pytorch.org', 'huggingface.co', 'SBOM'
];

export async function verifyReleasePolicy(root) {
  const failures = [];
  const [license, notice, thirdPartyNotices, packageJson, cargoToml] = await Promise.all([
    read(root, 'LICENSE'), read(root, 'NOTICE'), read(root, 'THIRD_PARTY_NOTICES.md'),
    read(root, 'package.json'), read(root, 'Cargo.toml')
  ]);
  if (!license.startsWith('MIT License')) failures.push('LICENSE must contain the MIT License text.');
  if (!notice.includes('Copyright (c) 2026 Firas Jaber')) failures.push('NOTICE must identify Firas Jaber as the Aywa copyright owner.');
  if (JSON.parse(packageJson).license !== 'MIT') failures.push('package.json must declare license "MIT".');
  if (!/^license = "MIT"$/m.test(cargoToml)) failures.push('Cargo.toml must declare license "MIT".');
  if (!/^repository = "https:\/\/github\.com\/firasjaber\/aywa"$/m.test(cargoToml)) failures.push('Cargo.toml must declare the Aywa repository URL.');
  for (const phrase of requiredNoticePhrases) if (!thirdPartyNotices.includes(phrase)) failures.push(`THIRD_PARTY_NOTICES.md must mention "\${phrase}".`);
  return failures;
}
```

- [ ] **Step 4: Run the verifier tests**

Run: `node --test scripts/verify-release-policy.test.mjs`

Expected: PASS with two tests passing.

- [ ] **Step 5: Commit the verifier**

```powershell
git add scripts/verify-release-policy.mjs scripts/verify-release-policy.test.mjs
git commit -m "test: verify release policy metadata"
```

### Task 2: Add MIT license, metadata, and distribution policy

**Files:**

- Create: `LICENSE`
- Create: `NOTICE`
- Create: `THIRD_PARTY_NOTICES.md`
- Modify: `package.json`
- Modify: `src-tauri/Cargo.toml`

- [ ] **Step 1: Add canonical MIT artifacts**

Create root `LICENSE` using the canonical MIT text with `Copyright (c) 2026 Firas Jaber`. Create root `NOTICE`:

```text
Aywa
Copyright (c) 2026 Firas Jaber

Aywa-owned source code and documentation are licensed under the MIT License.
See LICENSE for the full text. Third-party components are governed by their
own terms; see THIRD_PARTY_NOTICES.md.
```

- [ ] **Step 2: Write the initial third-party inventory and release gate**

Create `THIRD_PARTY_NOTICES.md` with sections named `Scope`, `Bundled components`, `First-run downloads`, `Network endpoints`, and `Release gate`. Its bundled-components table must include:

```markdown
| Component | Version or integrity record | Source | Delivery | Terms / release status |
| --- | --- | --- | --- | --- |
| CPython embeddable distribution | 3.12.10, SHA-256 `4acbed6dd1c744b0376e3b1cf57ce906f9dc9e95e68824584c8099a63025a3c3` | `https://www.python.org/ftp/python/3.12.10/python-3.12.10-embed-amd64.zip` | Bundled in the Windows application | Python Software Foundation License; retain notices and confirm redistribution terms before installer release. |
| get-pip.py | bundled pip 26.0.1, SHA-256 `feba1c697df45be1b539b40d93c102c9ee9dde1d966303323b830b06f3fbca3c` | `https://bootstrap.pypa.io/get-pip.py` | Bundled in the Windows application | Upstream pip/get-pip terms; retain notices and confirm redistribution terms before installer release. |
```

State that MIT does not relicense Rust/JavaScript dependencies, CPython, `get-pip.py`, PyTorch/CUDA, NeMo, Python wheels, or Hugging Face model weights. State that PyPI, `download.pytorch.org`, and `huggingface.co` content is downloaded at first run and cannot be directly redistributed until immutable versions/revisions, hashes, an SBOM, and a terms review are recorded.

- [ ] **Step 3: Update manifest metadata and command**

Add `"license": "MIT"` and `"verify:release-policy": "node scripts/verify-release-policy.mjs"` to `package.json`, preserving every existing script. Replace the blank Cargo fields:

```toml
license = "MIT"
repository = "https://github.com/firasjaber/aywa"
```

- [ ] **Step 4: Verify policy artifacts**

Run: `npm run verify:release-policy && node --test scripts/verify-release-policy.test.mjs`

Expected: no verifier output and two passing Node tests.

- [ ] **Step 5: Commit policy artifacts**

```powershell
git add LICENSE NOTICE THIRD_PARTY_NOTICES.md package.json src-tauri/Cargo.toml
git commit -m "docs: add MIT license and distribution policy"
```

### Task 3: Align public documentation and complete validation

**Files:**

- Modify: `README.md`

- [ ] **Step 1: Update repository references**

Replace every `https://github.com/firasjaber/stt` link in `README.md` with `https://github.com/firasjaber/aywa`. Update its clone example:

```powershell
git clone https://github.com/firasjaber/aywa.git
cd aywa
```

- [ ] **Step 2: Replace the temporary license warning**

Replace README's `## License` section:

```markdown
## License

Aywa-owned source code and documentation are licensed under the [MIT License](LICENSE). Third-party components, Python/runtime packages, and model weights remain subject to their own terms; see [third-party notices and distribution policy](THIRD_PARTY_NOTICES.md). Public installers are released only after their bundled components have completed the policy's provenance, notice, and redistribution review.
```

- [ ] **Step 3: Run the full validation set**

Run: `rg -n "firasjaber/stt|No software license has been selected|license = \\"\\"" README.md package.json src-tauri/Cargo.toml`

Expected: no matches.

Run: `npm run verify:release-policy && node --test scripts/verify-release-policy.test.mjs && npm run check`

Expected: all commands exit 0; Node reports two passing tests and Svelte reports 0 errors and 0 warnings.

- [ ] **Step 4: Commit documentation alignment**

```powershell
git add README.md
git commit -m "docs: document Aywa distribution policy"
```

- [ ] **Step 5: Inspect final state**

Run: `git diff --check HEAD~3..HEAD; git status --short`

Expected: no whitespace errors and a clean worktree.



