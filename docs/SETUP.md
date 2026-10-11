# Aywa setup

Aywa currently supports 64-bit Windows only. This guide covers both installing a packaged release and building from source.

## Install a release

There is no published installer yet. Once releases are available:

1. Visit [GitHub Releases](https://github.com/firasjaber/aywa/releases).
2. Download the unsigned Windows installer and its `SHA256SUMS.txt` file from the newest release.
3. Run the installer and start Aywa from the Start menu.
4. Follow the [non-technical user guide](USER_GUIDE.md) for first-time model setup and dictation.

Aywa installers are unsigned. Windows may show an unknown-publisher or SmartScreen warning. Download only from the project's GitHub Releases page and verify the SHA-256 checksum before running an installer; a checksum verifies the downloaded file's integrity, not the publisher's identity. See the [release checklist](RELEASE_CHECKLIST.md#verify-the-published-release).

### Runtime requirements

- 64-bit Windows 10 or Windows 11.
- A working microphone.
- Internet access for the first environment preparation and model download.
- Several gigabytes of free storage. One current model is approximately 2.5 GB, and Python, PyTorch, NeMo, caches, and temporary download copies require considerably more. Allow at least 15 GB during setup.
- CPU mode, or a supported NVIDIA GPU with a suitable driver for CUDA acceleration.

## Build from source

### Prerequisites

Install these Windows tools before cloning Aywa:

- Git.
- Node.js 20.19 or newer (or Node.js 22.12 or newer), with npm.
- Rust using `rustup`; the project pins Rust 1.95.0.
- Microsoft C++ Build Tools with the **Desktop development with C++** workload and a Windows SDK.
- Microsoft Edge WebView2 Runtime (included with current Windows versions).
- Python with pytest for running the small Python unit-test suite. The packaged app uses its own managed Python 3.12.10 runtime.

### Clone and install JavaScript dependencies

```powershell
git clone https://github.com/firasjaber/aywa.git
cd aywa
npm ci
```

### Prepare and run the managed Python build

```powershell
npm run tauri:dev:managed-python
```

This command verifies or downloads the locked embedded Python archive and `get-pip.py`, then starts the Tauri development app in managed-runtime mode. The downloaded resources are checksum-verified. On first model activation the app installs pinned PyTorch packages and NeMo dependencies from the internet; this can take several minutes and many gigabytes.

To prepare the embedded resources without starting the app:

```powershell
npm run prepare:python-runtime
```

Do not run `prepare:python-runtime:lock` during a normal build. That command intentionally replaces trusted checksums and is for maintainers updating the embedded runtime.

### Run checks and tests

```powershell
npm run check
npm run test
npm run build
node --test scripts/prepare-update-assets.test.mjs scripts/verify-release-policy.test.mjs
cargo test --manifest-path src-tauri/Cargo.toml --locked
python -m pytest src-tauri/python/tests -v
```

### Build a Windows installer

Distributed builds require the update signing key that matches the public key in `src-tauri/tauri.conf.json`. The private key is stored outside this repository at `%LOCALAPPDATA%\AywaRelease\updater.key` on the maintainer's machine. It must have a secure backup. Do not create a replacement key for normal builds: existing installations trust the configured public key.

```powershell
$env:TAURI_SIGNING_PRIVATE_KEY = Join-Path $env:LOCALAPPDATA 'AywaRelease\updater.key'
$env:TAURI_SIGNING_PRIVATE_KEY_PASSWORD = ''
npm run tauri:build
Remove-Item Env:TAURI_SIGNING_PRIVATE_KEY
Remove-Item Env:TAURI_SIGNING_PRIVATE_KEY_PASSWORD
```

The generated key uses an empty password. If you encrypt it later, supply its password through your secret manager instead. The command prepares the locked embedded Python files, builds the frontend, compiles the Rust application, and creates the NSIS installer and its `.sig` file under `src-tauri\target\release\bundle\nsis`.

For a local development installer without a signing key, disable update artifacts with a temporary build override. Do not distribute this build as an in-app update:

```powershell
npm run prepare:python-runtime
$developmentConfig = Join-Path $env:TEMP 'aywa-development-build.json'
[System.IO.File]::WriteAllText($developmentConfig, '{"bundle":{"createUpdaterArtifacts":false}}')
npx.cmd --no-install tauri build --bundles nsis --config $developmentConfig
```

Development builds skip automatic update checks. Manual checks remain available. Production builds check at startup and once a day; the Rust app keeps the schedule running while its window is hidden.

Before distributing a build, follow the [release checklist](RELEASE_CHECKLIST.md). Configure `TAURI_SIGNING_PRIVATE_KEY` and, if needed, `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` as GitHub Actions repository secrets. The key secret must contain the contents of the matching private key file, rather than its local file path. Do not paste that value into source files, logs, or release assets.

The GitHub release workflow verifies a signed installer download, tests a fresh installation and an upgrade on an isolated Windows runner, then publishes all update assets together. Update signatures protect downloads used by Aywa's updater. The Windows installer still has no Authenticode publisher certificate.
