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
npm run build
cargo test --manifest-path src-tauri/Cargo.toml
python -m pytest src-tauri/python/tests -v
```

### Build a Windows installer

```powershell
npm run tauri:build
```

The command prepares the locked embedded Python files, builds the frontend, compiles the Rust application, and creates Tauri bundles under `src-tauri\target\release\bundle`.

Before distributing a build, follow the [release checklist](RELEASE_CHECKLIST.md). The GitHub release workflow tests the build, publishes a SHA-256 hash, and smoke-installs the unsigned installer on a fresh Windows runner.
