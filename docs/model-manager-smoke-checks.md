# Model Manager Smoke Checks

## Direct Rust probe flow

Run the GPU preflight:

```powershell
powershell -ExecutionPolicy Bypass -File scripts/model-manager/probe.ps1 preflight
```

Expected: JSON showing `supported` and, on a healthy NVIDIA machine, a populated `gpu_name`.

Fetch the English model metadata:

```powershell
powershell -ExecutionPolicy Bypass -File scripts/model-manager/probe.ps1 manifest parakeet-v0.2-en
```

Expected: JSON containing `parakeet-tdt-0.6b-v2.nemo` and SHA `d99e39955c9d3d0350d8fb7c75e40c64a2b2eaeb003883d7c941fd2e8747b28c`.

## Tauri app flow

Run the backend tests:

```powershell
C:\Users\FIRRJ\.cargo\bin\cargo.exe test --manifest-path src-tauri/Cargo.toml -- --nocapture
```

Run the frontend check:

```powershell
npm run check
```

Launch the app:

```powershell
npm run tauri dev
```

Manual checklist:

1. Fresh install shows both models.
2. English is selected by default.
3. Select does not auto-download.
4. Download only becomes `Downloaded` after checksum verification.
5. Load is blocked when preflight fails.
6. Loading one model leaves only one `Loaded` row.
7. Unload keeps the model on disk.
