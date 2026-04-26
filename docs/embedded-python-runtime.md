# Embedded Python Runtime

The published Windows app ships the official CPython embeddable distribution as a Tauri resource. On first runtime bootstrap, the app extracts it into `%LOCALAPPDATA%/com.ok.speech/runtime/python`, enables `site` in the embedded `._pth` file, bootstraps `pip` with the bundled `get-pip.py`, then installs the selected runtime profile packages.

The app does not bundle `torch`, `torchaudio`, `nemo_toolkit`, CUDA wheels, or transcription models. Those are still installed or downloaded by the runtime profile and model manager flows.

## Build Prep

Run this before a release build:

```powershell
npm run prepare:python-runtime
```

Then build the app:

```powershell
npm run tauri:build
```

## Development Managed Runtime Test

Run the app in development while forcing the same managed runtime path used by published builds:

```powershell
npm run tauri:dev:managed-python
```

This sets `OK_PYTHON_RUNTIME=managed`, which disables `OK_PYTHON_EXE` and PATH fallback. If `src-tauri/resources/python` is missing, startup fails instead of silently using system Python.

## Developer Overrides

Set `OK_PYTHON_EXE` to force a specific interpreter:

```powershell
$env:OK_PYTHON_EXE = "C:\Python312\python.exe"
```

Unset it to use normal runtime resolution:

```powershell
Remove-Item Env:OK_PYTHON_EXE
```

Do not set `OK_PYTHON_EXE` when running `npm run tauri:dev:managed-python`; managed mode intentionally ignores it.

## Refreshing The Managed Runtime

Close the app, delete `%LOCALAPPDATA%/com.ok.speech/runtime/python`, and start the app again. The next bootstrap extracts a fresh copy from the bundled resource.

## Verification

Use a machine or VM without Python on `PATH`. Start the published app, open Settings, and wait for runtime setup. The Settings page should report a ready Python transcription runtime without requiring a manual Python install.
