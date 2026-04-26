# Runtime Profile Smoke Checks

## Backend

- Run Rust tests:

```powershell
& "$env:USERPROFILE\.cargo\bin\cargo.exe" test -- --nocapture
```

- Run frontend type checks:

```powershell
npm run check
```

## Manual Settings Checks

1. Open Settings and confirm the `Advanced runtime` section renders below the runtime summary.
2. Confirm `Selection mode`, `Resolved auto profile`, `Active profile`, `Execution device`, and `Last smoke test` all render.
3. Confirm model `Load` actions are disabled while runtime provisioning or smoke testing is active.

## Auto Mode

1. Leave the runtime section in `Auto` mode.
2. Click `Re-detect`.
3. Confirm logs include:
   - `[runtime-profile] detect`
   - `[runtime-profile] resolve`
   - `[runtime-profile] smoke-test start`
   - `[runtime-profile] activate success`
4. Confirm the active profile becomes the best detected profile or `cpu`.

## Manual Mode

1. Switch to `Manual`.
2. Pick a runtime profile from the dropdown.
3. Confirm provisioning starts immediately and Settings stays responsive.
4. Click `Apply now` or `Test profile` to force a retest.
5. Confirm a failure stays visible in the runtime section and does not silently switch profiles.

## CPU Fallback

1. Force a CPU-compatible environment or pick the `cpu` profile in manual mode.
2. Confirm the runtime section reports `CPU fallback`.
3. Confirm logs include `[runtime-profile] activate success profile_id=cpu device=cpu_fallback`.

## Reset To Auto

1. From manual mode, click `Reset to auto`.
2. Confirm the selection mode returns to `auto`.
3. Confirm the next runtime reconcile uses manifest-driven selection again.

## Development Managed Runtime

1. Close the app.
2. Delete `%LOCALAPPDATA%/com.ok.speech/runtime/python`.
3. Run `npm run tauri:dev:managed-python`.
4. Confirm Settings shows runtime setup progressing through bundled Python preparation.
5. Confirm logs for model load include `python_exe=%LOCALAPPDATA%/com.ok.speech/runtime/python/python.exe`.
6. Switch between Auto and Manual runtime profiles and confirm profile install and smoke-test behavior still works.
7. Load a downloaded model and run a short transcription.

## Clean Machine Without System Python

1. Use a Windows VM or machine where `python --version` fails in a normal terminal.
2. Install the published app build.
3. Start the app and open Settings.
4. Confirm runtime setup reaches `ready` without manually installing Python.
5. Confirm `%LOCALAPPDATA%/com.ok.speech/runtime/python/python.exe` exists.
6. Confirm the selected runtime profile installs packages and completes the smoke test.
7. Load a downloaded model and run a short transcription.
