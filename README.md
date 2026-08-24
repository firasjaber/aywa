# Aywa

Aywa is a Windows desktop app for local voice typing. Hold a global shortcut, speak, and release it; Aywa transcribes the recording on your computer, copies the result to the clipboard, and attempts to paste it into the active app.

> **Early development:** Aywa currently supports Windows only. Official installers are published unsigned through GitHub Releases and accompanied by SHA-256 checksums.

## What Aywa does

- Runs speech recognition locally after the required runtime and model files are installed.
- Supports English and multilingual NVIDIA Parakeet models.
- Uses `Ctrl` + `Alt` + `Space` as the default hold-to-talk shortcut and lets you change it in Settings.
- Automatically selects a compatible CPU or NVIDIA CUDA runtime.
- Lives in the Windows system tray when its Settings window is closed.
- Provides runtime, GPU, model, and log diagnostics for support.

Aywa contacts Python, PyPI/PyTorch, and Hugging Face services while preparing its local runtime and downloading models. Audio transcription itself is performed locally. See the [public-release audit](docs/PUBLIC_RELEASE_AUDIT.md) for current privacy limitations, including retained recordings and diagnostic logs.

Aywa keeps a WAV only while it is needed for local transcription, then deletes it. If a forced shutdown leaves a WAV behind, Aywa removes it at next startup. Diagnostics are stored locally in one log per launch and retained up to 1 GB total; the oldest sessions are removed first.

## Get started

Aywa does not have a published installer yet. When releases are available:

1. Open [GitHub Releases](https://github.com/firasjaber/aywa/releases).
2. Download the unsigned Windows installer and its `SHA256SUMS.txt` file.
3. Verify the installer hash before opening it; checksums verify file integrity, not the publisher's identity.
3. Follow the [user guide](docs/USER_GUIDE.md) to prepare Aywa, download a model, and dictate your first text.

For installation details and source builds, read the [setup guide](docs/SETUP.md).

## Documentation

- [User guide](docs/USER_GUIDE.md) — non-technical installation and everyday use.
- [Setup guide](docs/SETUP.md) — release installation and building from source.
- [Troubleshooting](docs/TROUBLESHOOTING.md) — symptom-based recovery steps.
- [Help and issue reports](docs/SUPPORT.md) — collect diagnostics and report a bug safely.
- [Security policy](SECURITY.md) — report a vulnerability privately.
- [Release checklist](docs/RELEASE_CHECKLIST.md) — verify releases and create a version tag.
- [Public-release audit](docs/PUBLIC_RELEASE_AUDIT.md) — prioritized release risks and recommendations.

## Build from source

Aywa uses Svelte 5 and TypeScript for its interface, Tauri 2 and Rust for the desktop application, and a managed Python runtime for NVIDIA NeMo transcription.

```powershell
git clone https://github.com/firasjaber/aywa.git
cd aywa
npm ci
npm run tauri:dev:managed-python
```

The first managed-runtime preparation downloads a Python distribution. Activating a model later installs substantial PyTorch and transcription dependencies. Review the complete [developer setup](docs/SETUP.md#build-from-source) before starting.

## Verify a checkout

```powershell
npm run check
npm run build
cargo test --manifest-path src-tauri/Cargo.toml
python -m pytest src-tauri/python/tests -v
```

## Support

Check [troubleshooting](docs/TROUBLESHOOTING.md) first. If the problem remains, follow the [support guide](docs/SUPPORT.md) and [open a bug report](https://github.com/firasjaber/aywa/issues/new?template=bug_report.yml).

## License

Aywa-owned source code and documentation are licensed under the [MIT License](LICENSE). Third-party components, Python/runtime packages, and model weights remain subject to their own terms; see [third-party notices and distribution policy](THIRD_PARTY_NOTICES.md). Public installers are released only after their bundled components have completed the policy's provenance, notice, and redistribution review.
