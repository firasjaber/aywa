# Tag-Only Release Validation Design

## Purpose

Run Aywa's heavyweight Windows validation only for version tags, avoiding
embedded-runtime downloads on pull requests and branch pushes while ensuring a
published release is fully tested.

## Workflow design

Delete the standalone `.github/workflows/ci.yml` workflow. The existing
`Unsigned Windows Release` workflow remains the sole automation entry point and
triggers only for pushed `v*` tags.

Before frontend, Rust, and Python tests, it installs the pinned pytest test
dependency and runs `npm run prepare:python-runtime`. This creates the
checksum-verified Python resources that Tauri requires while compiling its
build script. The workflow then runs policy validation, frontend checks/build,
Rust tests, Python tests, NSIS packaging, checksum generation, a clean-runner
silent-install smoke test, and GitHub Release publication.

## Dependency ownership

Add `src-tauri/python/requirements-test.txt` containing an exact pytest
version. Both local contributors and GitHub Actions install it with
`python -m pip install --requirement src-tauri/python/requirements-test.txt`.
The production NeMo requirements file remains separate so CI does not install
model/runtime packages merely to test worker protocol helpers.

## Validation and failure behavior

Each tag job fails before publishing if Python resource preparation, any test,
packaging, hash generation, or the installer smoke test fails. No workflow runs
for ordinary branch pushes or pull requests. The release checklist documents
that local checks should be run before tagging because GitHub will validate only
after the release tag is pushed.

## Scope

Keep unsigned installer disclosure, checksum publication, Windows-only NSIS
targets, and version/tag enforcement. Do not add branch CI, signing, or managed
runtime package installation.
