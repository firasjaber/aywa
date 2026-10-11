# Aywa

Fast, private voice transcription that runs entirely on your PC. Speak naturally and turn your words into text, with your audio processed locally and kept off the cloud.

Aywa runs on Windows. Hold `Ctrl` + `Alt` + `Space`, speak, and release it. Aywa copies the text to your clipboard and tries to paste it into the active app. If pasting fails, press `Ctrl` + `V`.

> Aywa is in early development. Windows installers do not have a verified publisher certificate. Release downloads include SHA-256 checksums.

## What Aywa does

- Transcribes speech locally after the initial setup and model download.
- Supports English and multilingual speech with two available Parakeet models.
- Lets you change the recording shortcut in Settings.
- Applies your word replacement and deletion rules before saving or pasting text.
- Saves transcription text in local History by default. You can turn this off or delete saved entries.
- Selects a compatible runtime for your CPU or supported NVIDIA graphics card.
- Runs in the Windows system tray when you close Settings.
- Checks for updates at startup and once a day, with installation controls in the window footer.

When you activate a Parakeet model, Aywa automatically prepares everything it needs to run in the background. You do not need to install Python, dependencies, or configure the environment yourself. Choose, download, and activate a model in the app, then wait for **Ready to dictate**. The first setup needs an internet connection and can take several minutes. Aywa reuses the prepared environment on later launches.

## Privacy

Aywa processes audio on your PC. It does not upload recordings or transcription history.

Temporary audio files are deleted after transcription. Aywa also removes recordings left by an interrupted session at the next startup. Transcription text stays in History until you delete it. To stop saving new text, turn off **Save transcriptions to History** in Settings.

Setup downloads Python, transcription dependencies, and models from Python, PyPI/PyTorch, and Hugging Face services. Aywa also contacts GitHub for update checks and downloads. Transcription runs locally once setup is complete.

Diagnostic logs stay on your PC, with a limit of 1 GB total. Aywa deletes the oldest logs first. See the [user guide](docs/USER_GUIDE.md#privacy-and-diagnostics) for details.

## Get started

For a packaged Windows installer, check [GitHub Releases](https://github.com/firasjaber/aywa/releases). If no release is available, follow the source build instructions below.

1. Open [GitHub Releases](https://github.com/firasjaber/aywa/releases).
2. Download the unsigned Windows installer and its `SHA256SUMS.txt` file.
3. Compare the installer hash with `SHA256SUMS.txt` before opening it. A matching checksum confirms file integrity, not publisher identity.
4. Follow the [user guide](docs/USER_GUIDE.md) to prepare Aywa, download a model, and dictate your first text.

For later updates, select **Download and install** in the footer. Aywa restarts and keeps your settings, rules, History, and downloaded files. See [Updating Aywa](docs/USER_GUIDE.md#update-aywa) for details and instructions for older versions.

For installation details and source builds, read the [setup guide](docs/SETUP.md).

## Documentation

- [User guide](docs/USER_GUIDE.md) — non-technical installation and everyday use.
- [Setup guide](docs/SETUP.md) — release installation and building from source.
- [Troubleshooting](docs/TROUBLESHOOTING.md) — symptom-based recovery steps.
- [Help and issue reports](docs/SUPPORT.md) — collect diagnostics and report a bug safely.
- [Security policy](SECURITY.md) — report a vulnerability privately.
- [Release checklist](docs/RELEASE_CHECKLIST.md) — verify releases and create a version tag.
- [Public-release audit](docs/PUBLIC_RELEASE_AUDIT.md) — historical findings from July 2026, not a current description of the app.

## Build from source

Aywa uses Svelte 5 and TypeScript for its interface, Tauri 2 and Rust for the desktop application, and a managed Python runtime for NVIDIA NeMo transcription.

Install the [build prerequisites](docs/SETUP.md#prerequisites) first, including Node.js 22.15.0 or newer and the Windows C++ build tools.

```powershell
git clone https://github.com/firasjaber/aywa.git
cd aywa
npm ci
npm run tauri:dev:managed-python
```

The first run downloads Python. Activating a model downloads additional dependencies and model files. Setup can take several minutes and needs at least 15 GB of free storage. Read the [setup guide](docs/SETUP.md#build-from-source) for details.

## Verify a checkout

```powershell
npm run check
npm run test
npm run build
node --test scripts/prepare-update-assets.test.mjs scripts/verify-release-policy.test.mjs
cargo test --manifest-path src-tauri/Cargo.toml --locked
python -m pytest src-tauri/python/tests -v
```

## Support

Check [troubleshooting](docs/TROUBLESHOOTING.md) first. If the problem remains, follow the [support guide](docs/SUPPORT.md) and [open a bug report](https://github.com/firasjaber/aywa/issues/new?template=bug_report.yml).

## License

Aywa source code and documentation use the [MIT License](LICENSE). Third-party components and model weights have their own license terms. See [third-party notices and distribution policy](THIRD_PARTY_NOTICES.md) for details and installer release requirements.
