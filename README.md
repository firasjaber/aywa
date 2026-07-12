# Aywa

Aywa is a Windows desktop app for local voice typing. Hold a global shortcut, speak, and release it; Aywa transcribes the recording on your computer, copies the result to the clipboard, and attempts to paste it into the active app.

> **Early development:** Aywa currently supports Windows only. Public release installers, code signing, and a software license have not been finalized yet.

## What Aywa does

- Runs speech recognition locally after the required runtime and model files are installed.
- Supports English and multilingual NVIDIA Parakeet models.
- Uses `Ctrl` + `Alt` + `Space` as a hold-to-talk shortcut.
- Automatically selects a compatible CPU or NVIDIA CUDA runtime.
- Lives in the Windows system tray when its Settings window is closed.
- Provides runtime, GPU, model, and log diagnostics for support.

Aywa contacts Python, PyPI/PyTorch, and Hugging Face services while preparing its local runtime and downloading models. Audio transcription itself is performed locally. See the [public-release audit](docs/PUBLIC_RELEASE_AUDIT.md) for current privacy limitations, including retained recordings and diagnostic logs.

## Get started

Aywa does not have a published installer yet. When releases are available:

1. Open [GitHub Releases](https://github.com/firasjaber/stt/releases).
2. Download and install the Windows release.
3. Follow the [user guide](docs/USER_GUIDE.md) to prepare Aywa, download a model, and dictate your first text.

For installation details and source builds, read the [setup guide](docs/SETUP.md).

## Documentation

- [User guide](docs/USER_GUIDE.md) — non-technical installation and everyday use.
- [Setup guide](docs/SETUP.md) — release installation and building from source.
- [Troubleshooting](docs/TROUBLESHOOTING.md) — symptom-based recovery steps.
- [Help and issue reports](docs/SUPPORT.md) — collect diagnostics and report a bug safely.
- [Public-release audit](docs/PUBLIC_RELEASE_AUDIT.md) — prioritized release risks and recommendations.

## Build from source

Aywa uses Svelte 5 and TypeScript for its interface, Tauri 2 and Rust for the desktop application, and a managed Python runtime for NVIDIA NeMo transcription.

```powershell
git clone https://github.com/firasjaber/stt.git
cd stt
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

Check [troubleshooting](docs/TROUBLESHOOTING.md) first. If the problem remains, follow the [support guide](docs/SUPPORT.md) and [open a bug report](https://github.com/firasjaber/stt/issues/new?template=bug_report.yml).

## License

No software license has been selected yet. Until a license file is added, the source is visible but no permission to use, modify, or redistribute it is granted. Resolving this is a blocker before the intended public release.
