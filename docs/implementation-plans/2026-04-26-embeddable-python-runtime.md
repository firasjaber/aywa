# Embeddable Python Runtime Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Ship a managed Windows Python runtime with the Tauri application so non-technical users do not need to install Python, while continuing to install CPU/GPU ML packages from runtime profiles on demand.

**Architecture:** Add a small runtime installer around the official Windows embeddable CPython zip. At app runtime, prefer `OK_PYTHON_EXE` for development, otherwise ensure `app_data/runtime/python/python.exe` exists by extracting the bundled zip, enabling `site`, bootstrapping `pip`, and then reusing the existing runtime-profile package installation and smoke-test flow.

**Tech Stack:** Tauri v2, Rust, official CPython Windows embeddable distribution, `zip` crate, PowerShell release-prep script, existing pip/runtime-profile manager, existing Python worker scripts

---

## Scope Check

This is one subsystem: app-managed Python interpreter provisioning. It intentionally does not package `torch`, `torchaudio`, `nemo_toolkit`, CUDA wheels, or transcription models into the installer. Runtime profile package installation remains pip-based and profile-driven.

## Decisions Locked In

1. Do not use PyInstaller for this version.
2. Bundle the official Windows embeddable CPython zip as a Tauri resource.
3. Extract Python into `app_data/runtime/python` because the bundled install must be writable for `pip` and site packages.
4. Keep `OK_PYTHON_EXE` as the first candidate for developer/debug overrides.
5. Keep `python` on `PATH` as a development fallback when the bundled zip resource is not present.
6. Bootstrap `pip` with a pinned `get-pip.py` resource because the embeddable CPython distribution does not include `ensurepip`.
7. Keep current runtime profiles responsible for installing `torch`/`torchaudio` CPU or CUDA packages.
8. Treat Python runtime version changes as reinstall events for `app_data/runtime/python`.
9. Add `OK_PYTHON_RUNTIME=managed` as a development/test mode that disables `OK_PYTHON_EXE` and PATH fallback, forcing the app to use the extracted embeddable runtime.
10. Add a dedicated npm script for running Tauri dev against the managed runtime so the same runtime path can be tested before publishing.

## File Map

### Release/runtime artifacts

- Create: `src-tauri/python-runtime/runtime-manifest.json`
  Defines the CPython embeddable zip URL, filename, `get-pip.py` URL, and filename.
- Create generated lock file: `src-tauri/python-runtime/runtime-lock.json`
  Stores the exact SHA-256 values produced by the prep script and committed with the source tree.
- Create: `scripts/prepare-python-runtime.ps1`
  Downloads and verifies the CPython embeddable zip and `get-pip.py` into Tauri resources before release builds.
- Create directory at build time: `src-tauri/resources/python/`
  Contains downloaded runtime artifacts. The files are generated, not committed.
- Modify: `.gitignore`
  Ignores generated Python runtime artifacts under `src-tauri/resources/python/`.
- Modify: `package.json`
  Adds scripts for preparing runtime artifacts, running dev with forced managed Python, and running a release Tauri build.
- Modify: `src-tauri/tauri.conf.json`
  Includes `resources/python/*` as bundle resources.

### Rust backend

- Modify: `src-tauri/Cargo.toml`
  Adds `zip` for runtime extraction and `tempfile` for tests.
- Create: `src-tauri/src/python_runtime.rs`
  Owns locating, installing, validating, and bootstrapping the app-managed Python interpreter, including forced managed-runtime development mode.
- Modify: `src-tauri/src/lib.rs`
  Exports `python_runtime` and updates `bootstrap_transcription_runtime` to prepare worker files and the interpreter.
- Modify: `src-tauri/src/transcribe.rs`
  Replaces path-only Python resolution with app-aware runtime resolution while preserving low-level command helpers.
- Modify: `src-tauri/src/runtime_profile/service.rs`
  Ensures the app-managed Python runtime is ready before probing, installing profile packages, or smoke testing.
- Modify: `src-tauri/src/model_manager/types.rs`
  Adds a bootstrap status for preparing the embedded Python runtime.

### Docs

- Create: `docs/embedded-python-runtime.md`
  Documents how the runtime is bundled, where it is installed, how to force refresh, and how to verify on a clean machine.
- Modify: `docs/runtime-profile-smoke-checks.md`
  Adds clean-machine checks for no system Python installed.

## Shared Names To Keep Consistent

```rust
pub const PYTHON_RUNTIME_VERSION: &str = "3.12.10-embed-amd64";
pub const PYTHON_RUNTIME_RESOURCE_DIR: &str = "python";
pub const PYTHON_RUNTIME_ZIP_NAME: &str = "python-3.12.10-embed-amd64.zip";
pub const GET_PIP_RESOURCE_NAME: &str = "get-pip.py";
pub const PYTHON_RUNTIME_MARKER_FILE: &str = "ok-python-runtime.json";
pub const PYTHON_RUNTIME_MODE_ENV: &str = "OK_PYTHON_RUNTIME";
pub const PYTHON_RUNTIME_MODE_MANAGED: &str = "managed";
```

```rust
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct PythonRuntimeMarker {
    pub version: String,
}
```

```rust
pub fn ensure_python_runtime<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    app_data_root: &std::path::Path,
) -> Result<std::path::PathBuf, String>;
```

## Task 1: Add Runtime Artifact Manifest And Prep Script

**Files:**
- Create: `src-tauri/python-runtime/runtime-manifest.json`
- Create: `scripts/prepare-python-runtime.ps1`
- Modify: `.gitignore`
- Modify: `package.json`

- [ ] **Step 1: Add the pinned runtime manifest**

Create `src-tauri/python-runtime/runtime-manifest.json`:

```json
{
  "pythonVersion": "3.12.10",
  "runtimeId": "3.12.10-embed-amd64",
  "pythonZip": {
    "fileName": "python-3.12.10-embed-amd64.zip",
    "url": "https://www.python.org/ftp/python/3.12.10/python-3.12.10-embed-amd64.zip"
  },
  "getPip": {
    "fileName": "get-pip.py",
    "url": "https://bootstrap.pypa.io/get-pip.py"
  }
}
```

- [ ] **Step 2: Add the download and checksum script**

Create `scripts/prepare-python-runtime.ps1`:

```powershell
param(
    [string]$ManifestPath = "src-tauri/python-runtime/runtime-manifest.json",
    [string]$LockPath = "src-tauri/python-runtime/runtime-lock.json",
    [string]$OutputDir = "src-tauri/resources/python",
    [switch]$UpdateLock
)

$ErrorActionPreference = "Stop"

function Get-Sha256([string]$Path) {
    return (Get-FileHash -Algorithm SHA256 -Path $Path).Hash.ToLowerInvariant()
}

function Save-File([string]$Url, [string]$Path) {
    if (!(Test-Path $Path)) {
        Invoke-WebRequest -Uri $Url -OutFile $Path
    }
}

function Assert-Checksum([string]$Path, [string]$ExpectedSha256) {
    $actual = Get-Sha256 $Path
    if ($actual -ne $ExpectedSha256.ToLowerInvariant()) {
        Remove-Item $Path -Force
        throw "Checksum mismatch for $Path. Expected $ExpectedSha256, got $actual."
    }
}

$manifest = Get-Content $ManifestPath -Raw | ConvertFrom-Json
New-Item -ItemType Directory -Force -Path $OutputDir | Out-Null

$pythonZipPath = Join-Path $OutputDir $manifest.pythonZip.fileName
$getPipPath = Join-Path $OutputDir $manifest.getPip.fileName

Save-File $manifest.pythonZip.url $pythonZipPath
Save-File $manifest.getPip.url $getPipPath

if ($UpdateLock) {
    $lock = [ordered]@{
        runtimeId = $manifest.runtimeId
        pythonZip = [ordered]@{
            fileName = $manifest.pythonZip.fileName
            sha256 = Get-Sha256 $pythonZipPath
        }
        getPip = [ordered]@{
            fileName = $manifest.getPip.fileName
            sha256 = Get-Sha256 $getPipPath
        }
    }
    $lock | ConvertTo-Json -Depth 5 | Set-Content -Path $LockPath -Encoding UTF8
    Write-Host "Updated Python runtime lock file at $LockPath"
}

if (!(Test-Path $LockPath)) {
    throw "Runtime lock file missing. Run npm run prepare:python-runtime:lock once and commit $LockPath."
}

$locked = Get-Content $LockPath -Raw | ConvertFrom-Json
Assert-Checksum $pythonZipPath $locked.pythonZip.sha256
Assert-Checksum $getPipPath $locked.getPip.sha256

Write-Host "Prepared Python runtime resources in $OutputDir"
```

- [ ] **Step 3: Ignore generated runtime resources**

Add these lines to `.gitignore`:

```gitignore

# Generated Python runtime bundle resources
/src-tauri/resources/python/*.zip
/src-tauri/resources/python/get-pip.py
```

- [ ] **Step 4: Add package scripts**

Modify `package.json` scripts:

```json
{
  "scripts": {
    "prepare:python-runtime:lock": "powershell -ExecutionPolicy Bypass -File scripts/prepare-python-runtime.ps1 -UpdateLock",
    "prepare:python-runtime": "powershell -ExecutionPolicy Bypass -File scripts/prepare-python-runtime.ps1",
    "tauri:dev:managed-python": "npm run prepare:python-runtime && set \"OK_PYTHON_RUNTIME=managed\" && npm run tauri -- dev",
    "tauri:build": "npm run prepare:python-runtime && npm run tauri -- build"
  }
}
```

Keep the existing scripts unchanged.

- [ ] **Step 5: Verify the prep script**

Run:

```powershell
npm run prepare:python-runtime:lock
npm run prepare:python-runtime
```

Expected: `src-tauri/python-runtime/runtime-lock.json`, `src-tauri/resources/python/python-3.12.10-embed-amd64.zip`, and `src-tauri/resources/python/get-pip.py` exist, and the script prints `Prepared Python runtime resources`.

- [ ] **Step 6: Verify managed-runtime dev script is present**

Run:

```powershell
npm run tauri:dev:managed-python -- --help
```

Expected: the command prepares Python runtime resources, sets `OK_PYTHON_RUNTIME=managed`, and prints Tauri CLI help without starting the app.

- [ ] **Step 7: Commit**

```powershell
git add .gitignore package.json scripts/prepare-python-runtime.ps1 src-tauri/python-runtime/runtime-manifest.json src-tauri/python-runtime/runtime-lock.json
git commit -m "add embedded python runtime artifact prep"
```

## Task 2: Bundle Runtime Resources With Tauri

**Files:**
- Modify: `src-tauri/tauri.conf.json`
- Modify: `src-tauri/Cargo.toml`

- [ ] **Step 1: Add Tauri resource inclusion**

Modify the `bundle` section in `src-tauri/tauri.conf.json`:

```json
"bundle": {
  "active": true,
  "targets": "all",
  "resources": [
    "resources/python/*"
  ],
  "icon": [
    "icons/32x32.png",
    "icons/128x128.png",
    "icons/128x128@2x.png",
    "icons/icon.icns",
    "icons/icon.ico"
  ]
}
```

- [ ] **Step 2: Add Rust dependencies**

Modify `src-tauri/Cargo.toml`:

```toml
[dependencies]
zip = { version = "2.2", default-features = false, features = ["deflate"] }

[dev-dependencies]
tempfile = "3"
```

Keep the existing dependencies in place.

- [ ] **Step 3: Verify dependency resolution**

Run:

```powershell
cargo check
```

Working directory: `src-tauri`

Expected: check completes or fails only because `python_runtime` has not been implemented yet if Task 3 has already started.

- [ ] **Step 4: Commit**

```powershell
git add src-tauri/Cargo.toml src-tauri/tauri.conf.json src-tauri/Cargo.lock
git commit -m "bundle embedded python runtime resources"
```

## Task 3: Implement Python Runtime Installer Module

**Files:**
- Create: `src-tauri/src/python_runtime.rs`
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: Write tests for marker and pth handling**

Create `src-tauri/src/python_runtime.rs` with the tests first:

```rust
use std::{fs, path::{Path, PathBuf}, process::{Command, Stdio}};

use tauri::Manager;

pub const PYTHON_RUNTIME_VERSION: &str = "3.12.10-embed-amd64";
pub const PYTHON_RUNTIME_ZIP_NAME: &str = "python-3.12.10-embed-amd64.zip";
pub const GET_PIP_RESOURCE_NAME: &str = "get-pip.py";
pub const PYTHON_RUNTIME_MARKER_FILE: &str = "ok-python-runtime.json";
pub const PYTHON_RUNTIME_MODE_ENV: &str = "OK_PYTHON_RUNTIME";
pub const PYTHON_RUNTIME_MODE_MANAGED: &str = "managed";

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct PythonRuntimeMarker {
    pub version: String,
}

fn marker_path(runtime_dir: &Path) -> PathBuf {
    runtime_dir.join(PYTHON_RUNTIME_MARKER_FILE)
}

fn write_marker(runtime_dir: &Path) -> Result<(), String> {
    let marker = PythonRuntimeMarker {
        version: PYTHON_RUNTIME_VERSION.to_string(),
    };
    let json = serde_json::to_string_pretty(&marker).map_err(|error| error.to_string())?;
    fs::write(marker_path(runtime_dir), json).map_err(|error| error.to_string())
}

fn marker_matches(runtime_dir: &Path) -> bool {
    let Ok(contents) = fs::read_to_string(marker_path(runtime_dir)) else {
        return false;
    };
    serde_json::from_str::<PythonRuntimeMarker>(&contents)
        .map(|marker| marker.version == PYTHON_RUNTIME_VERSION)
        .unwrap_or(false)
}

fn enable_site_in_pth(contents: &str) -> String {
    contents
        .lines()
        .map(|line| if line.trim() == "#import site" { "import site" } else { line })
        .collect::<Vec<_>>()
        .join("\n")
        + "\n"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn marker_matches_current_runtime_version() {
        let temp = tempfile::tempdir().unwrap();
        write_marker(temp.path()).unwrap();
        assert!(marker_matches(temp.path()));
    }

    #[test]
    fn marker_rejects_different_runtime_version() {
        let temp = tempfile::tempdir().unwrap();
        fs::write(
            marker_path(temp.path()),
            r#"{"version":"3.11.9-embed-amd64"}"#,
        )
        .unwrap();
        assert!(!marker_matches(temp.path()));
    }

    #[test]
    fn pth_patch_enables_import_site() {
        let patched = enable_site_in_pth("python312.zip\n.\n#import site\n");
        assert_eq!(patched, "python312.zip\n.\nimport site\n");
    }
}
```

- [ ] **Step 2: Run tests and verify they pass**

Run:

```powershell
cargo test python_runtime --lib
```

Working directory: `src-tauri`

Expected: all `python_runtime` tests pass.

- [ ] **Step 3: Implement extraction and zip-slip protection**

Add this code above the test module in `src-tauri/src/python_runtime.rs`:

```rust
fn extract_zip(zip_path: &Path, target_dir: &Path) -> Result<(), String> {
    let file = fs::File::open(zip_path).map_err(|error| error.to_string())?;
    let mut archive = zip::ZipArchive::new(file).map_err(|error| error.to_string())?;

    fs::create_dir_all(target_dir).map_err(|error| error.to_string())?;

    for index in 0..archive.len() {
        let mut entry = archive.by_index(index).map_err(|error| error.to_string())?;
        let enclosed = entry
            .enclosed_name()
            .ok_or_else(|| format!("zip entry has unsafe path: {}", entry.name()))?;
        let output_path = target_dir.join(enclosed);

        if entry.is_dir() {
            fs::create_dir_all(&output_path).map_err(|error| error.to_string())?;
            continue;
        }

        if let Some(parent) = output_path.parent() {
            fs::create_dir_all(parent).map_err(|error| error.to_string())?;
        }

        let mut output = fs::File::create(&output_path).map_err(|error| error.to_string())?;
        std::io::copy(&mut entry, &mut output).map_err(|error| error.to_string())?;
    }

    Ok(())
}

fn patch_embedded_python_pth(runtime_dir: &Path) -> Result<(), String> {
    let pth_path = fs::read_dir(runtime_dir)
        .map_err(|error| error.to_string())?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .find(|path| path.extension().is_some_and(|extension| extension == "_pth"))
        .ok_or_else(|| "embedded Python ._pth file missing".to_string())?;

    let contents = fs::read_to_string(&pth_path).map_err(|error| error.to_string())?;
    let patched = enable_site_in_pth(&contents);
    if patched != contents {
        fs::write(&pth_path, patched).map_err(|error| error.to_string())?;
    }

    Ok(())
}
```

- [ ] **Step 4: Add installer and bootstrap helpers**

Add this public API in `src-tauri/src/python_runtime.rs`:

```rust
pub fn python_runtime_dir(app_data_root: &Path) -> PathBuf {
    app_data_root.join("runtime").join("python")
}

pub fn python_executable(app_data_root: &Path) -> PathBuf {
    python_runtime_dir(app_data_root).join("python.exe")
}

fn is_usable_python(candidate: &Path) -> bool {
    Command::new(candidate)
        .arg("--version")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

fn run_python_command(python_exe: &Path, args: &[&str], description: &str) -> Result<(), String> {
    let output = Command::new(python_exe)
        .args(args)
        .stdin(Stdio::null())
        .output()
        .map_err(|error| format!("failed to {description}: {error}"))?;

    if output.status.success() {
        return Ok(());
    }

    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    Err(if stderr.is_empty() {
        format!("failed to {description}")
    } else {
        format!("failed to {description}: {stderr}")
    })
}

fn bootstrap_pip(python_exe: &Path, get_pip_path: &Path) -> Result<(), String> {
    if Command::new(python_exe)
        .args(["-m", "pip", "--version"])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
    {
        return Ok(());
    }

    run_python_command(
        python_exe,
        &[
            get_pip_path
                .to_str()
                .ok_or_else(|| "get-pip path is not valid UTF-8".to_string())?,
            "--no-warn-script-location",
        ],
        "bootstrap pip",
    )
}
```

- [ ] **Step 5: Add app resource resolution**

Add this code to `src-tauri/src/python_runtime.rs`:

```rust
fn configured_python_override() -> Option<PathBuf> {
    std::env::var_os("OK_PYTHON_EXE")
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
}

fn path_python() -> PathBuf {
    PathBuf::from("python")
}

fn forced_managed_runtime() -> bool {
    std::env::var_os(PYTHON_RUNTIME_MODE_ENV)
        .and_then(|value| value.into_string().ok())
        .map(|value| value.eq_ignore_ascii_case(PYTHON_RUNTIME_MODE_MANAGED))
        .unwrap_or(false)
}

fn python_resource_dirs<R: tauri::Runtime>(app: &tauri::AppHandle<R>) -> Result<Vec<PathBuf>, String> {
    let mut dirs = vec![app
        .path()
        .resource_dir()
        .map_err(|error| error.to_string())?
        .join("python")];

    #[cfg(debug_assertions)]
    if let Some(manifest_dir) = option_env!("CARGO_MANIFEST_DIR") {
        dirs.push(PathBuf::from(manifest_dir).join("resources").join("python"));
    }

    Ok(dirs)
}

fn find_runtime_resources<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
) -> Result<Option<(PathBuf, PathBuf)>, String> {
    for resource_dir in python_resource_dirs(app)? {
        let zip_path = resource_dir.join(PYTHON_RUNTIME_ZIP_NAME);
        let get_pip_path = resource_dir.join(GET_PIP_RESOURCE_NAME);
        if zip_path.exists() && get_pip_path.exists() {
            return Ok(Some((zip_path, get_pip_path)));
        }
    }

    Ok(None)
}

pub fn ensure_python_runtime<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    app_data_root: &Path,
) -> Result<PathBuf, String> {
    let force_managed = forced_managed_runtime();

    if !force_managed {
        if let Some(override_path) = configured_python_override() {
            if is_usable_python(&override_path) {
                return Ok(override_path);
            }
            return Err(format!("OK_PYTHON_EXE is not usable: {}", override_path.display()));
        }
    }

    let runtime_dir = python_runtime_dir(app_data_root);
    let python_exe = python_executable(app_data_root);
    if marker_matches(&runtime_dir) && is_usable_python(&python_exe) {
        return Ok(python_exe);
    }

    if let Some((zip_path, get_pip_path)) = find_runtime_resources(app)? {
        if runtime_dir.exists() {
            fs::remove_dir_all(&runtime_dir).map_err(|error| error.to_string())?;
        }
        extract_zip(&zip_path, &runtime_dir)?;
        patch_embedded_python_pth(&runtime_dir)?;
        bootstrap_pip(&python_exe, &get_pip_path)?;
        write_marker(&runtime_dir)?;
        return Ok(python_exe);
    }

    if force_managed {
        return Err(
            "OK_PYTHON_RUNTIME=managed was requested, but bundled Python resources were not found. Run npm run prepare:python-runtime."
                .to_string(),
        );
    }

    let fallback = path_python();
    if is_usable_python(&fallback) {
        return Ok(fallback);
    }

    Err("No usable Python runtime found. Set OK_PYTHON_EXE or run npm run prepare:python-runtime before building."
        .to_string())
}
```

- [ ] **Step 6: Export the module**

Add to `src-tauri/src/lib.rs` near the other module exports:

```rust
pub mod python_runtime;
```

- [ ] **Step 7: Run tests**

Run:

```powershell
cargo test python_runtime --lib
```

Working directory: `src-tauri`

Expected: all `python_runtime` tests pass.

- [ ] **Step 8: Commit**

```powershell
git add src-tauri/src/python_runtime.rs src-tauri/src/lib.rs src-tauri/Cargo.toml src-tauri/Cargo.lock
git commit -m "add app-managed embedded python installer"
```

## Task 4: Wire Runtime Installer Into Transcription Bootstrap

**Files:**
- Modify: `src-tauri/src/model_manager/types.rs`
- Modify: `src-tauri/src/runtime_profile/service.rs`
- Modify: `src-tauri/src/transcribe.rs`
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: Add a bootstrap status for Python extraction**

Modify `WorkerRuntimeBootstrapStatus` in `src-tauri/src/model_manager/types.rs`:

```rust
pub enum WorkerRuntimeBootstrapStatus {
    Idle,
    Checking,
    InstallingPythonRuntime,
    UpgradingPip,
    InstallingCudaTorch,
    InstallingWorkerRequirements,
    FallingBackToCpu,
    SmokeTesting,
    Ready,
    Error,
}
```

- [ ] **Step 2: Replace root-only Python readiness with app-aware readiness**

In `src-tauri/src/transcribe.rs`, replace:

```rust
fn ensure_worker_python_ready(app_data_root: &Path) -> Result<PathBuf, String> {
    crate::runtime_profile::service::ensure_worker_python_ready(app_data_root)
}
```

with:

```rust
fn ensure_worker_python_ready<R: Runtime>(app: &AppHandle<R>) -> Result<PathBuf, String> {
    crate::runtime_profile::service::ensure_worker_python_ready(app)
}
```

- [ ] **Step 3: Update callers in `transcribe.rs`**

Change `probe_worker_cuda`:

```rust
pub fn probe_worker_cuda<R: Runtime>(app: &AppHandle<R>) -> Result<WorkerCudaStatus, String> {
    let python_exe = ensure_worker_python_ready(app)?;
    probe_worker_cuda_with_python(&python_exe)
}
```

Change `ensure_loaded`:

```rust
pub fn ensure_loaded<R: Runtime>(
    app: &AppHandle<R>,
    model_id: ModelId,
    model_path: &Path,
) -> Result<(), String> {
    let root = app_data_dir(app)?;
    let python_exe = ensure_worker_python_ready(app)?;
    // keep the rest of the function unchanged
}
```

- [ ] **Step 4: Update runtime-profile service public API**

In `src-tauri/src/runtime_profile/service.rs`, replace the existing root-only helper:

```rust
pub fn ensure_worker_python_ready(app_data_root: &Path) -> Result<PathBuf, String> {
    let prepared = prepare_worker_runtime_at_root(app_data_root, |_, _, _| {})?;
    Ok(prepared.python_exe)
}
```

with:

```rust
pub fn ensure_worker_python_ready<R: Runtime>(app: &AppHandle<R>) -> Result<PathBuf, String> {
    let root = app_data_dir(app)?;
    let prepared = prepare_worker_runtime_for_app(app, &root, |_, _, _| {})?;
    Ok(prepared.python_exe)
}
```

- [ ] **Step 5: Split app-aware preparation from root-only preparation**

In `src-tauri/src/runtime_profile/service.rs`, keep `prepare_worker_runtime_at_root` for tests and add this wrapper:

```rust
fn prepare_worker_runtime_for_app<R, F>(
    app: &AppHandle<R>,
    app_data_root: &Path,
    mut progress: F,
) -> Result<PreparedWorkerRuntime, String>
where
    R: Runtime,
    F: FnMut(WorkerRuntimeBootstrapStatus, Option<String>, Option<WorkerCudaStatus>),
{
    progress(
        WorkerRuntimeBootstrapStatus::InstallingPythonRuntime,
        Some("Preparing bundled Python runtime".to_string()),
        None,
    );
    let python_exe = crate::python_runtime::ensure_python_runtime(app, app_data_root)?;
    prepare_worker_runtime_with_python_at_root(app_data_root, python_exe, progress)
}
```

Rename the existing body of `prepare_worker_runtime_at_root` to accept a prepared interpreter:

```rust
fn prepare_worker_runtime_with_python_at_root<F>(
    app_data_root: &Path,
    python_exe: PathBuf,
    mut progress: F,
) -> Result<PreparedWorkerRuntime, String>
where
    F: FnMut(WorkerRuntimeBootstrapStatus, Option<String>, Option<WorkerCudaStatus>),
{
    progress(
        WorkerRuntimeBootstrapStatus::Checking,
        Some("Checking Python transcription runtime".to_string()),
        None,
    );

    transcribe::bootstrap_runtime(app_data_root)?;
    let manifest = catalog::load_manifest().map_err(|error| error.to_string())?;
    // keep the existing runtime-profile preparation logic after the old verify_runtime_layout call
}
```

Then make `prepare_worker_runtime_at_root` call the old path-based resolver for tests/dev:

```rust
fn prepare_worker_runtime_at_root<F>(
    app_data_root: &Path,
    progress: F,
) -> Result<PreparedWorkerRuntime, String>
where
    F: FnMut(WorkerRuntimeBootstrapStatus, Option<String>, Option<WorkerCudaStatus>),
{
    transcribe::bootstrap_runtime(app_data_root)?;
    let python_exe = transcribe::verify_runtime_layout(app_data_root)?;
    prepare_worker_runtime_with_python_at_root(app_data_root, python_exe, progress)
}
```

- [ ] **Step 6: Update `prepare_worker_runtime` to use app-aware preparation**

In `src-tauri/src/runtime_profile/service.rs`:

```rust
pub fn prepare_worker_runtime<R, F>(app: &AppHandle<R>, progress: F) -> Result<WorkerCudaStatus, String>
where
    R: Runtime,
    F: FnMut(WorkerRuntimeBootstrapStatus, Option<String>, Option<WorkerCudaStatus>),
{
    let root = app_data_dir(app)?;
    let prepared = prepare_worker_runtime_for_app(app, &root, progress)?;
    Ok(prepared.worker_cuda_status)
}
```

- [ ] **Step 7: Update library bootstrap**

Modify `bootstrap_transcription_runtime` in `src-tauri/src/lib.rs`:

```rust
pub fn bootstrap_transcription_runtime<R: Runtime>(app: &AppHandle<R>) -> Result<(), String> {
    let root = app_data_dir(app)?;
    transcribe::bootstrap_runtime(&root)?;
    let _python_exe = python_runtime::ensure_python_runtime(app, &root)?;
    Ok(())
}
```

- [ ] **Step 8: Run Rust tests**

Run:

```powershell
cargo test --lib
```

Working directory: `src-tauri`

Expected: all Rust library tests pass.

- [ ] **Step 9: Commit**

```powershell
git add src-tauri/src/model_manager/types.rs src-tauri/src/runtime_profile/service.rs src-tauri/src/transcribe.rs src-tauri/src/lib.rs
git commit -m "use bundled python runtime for transcription bootstrap"
```

## Task 5: Harden Pip Installation For Managed Runtime

**Files:**
- Modify: `src-tauri/src/runtime_profile/provision.rs`
- Modify: `src-tauri/src/transcribe.rs`

- [ ] **Step 1: Add non-interactive pip flags to profile installs**

Modify `build_pip_install_args` in `src-tauri/src/runtime_profile/provision.rs`:

```rust
pub fn build_pip_install_args(profile: &RuntimeProfileCatalogEntry) -> Vec<String> {
    let mut args = vec![
        "-m".to_string(),
        "pip".to_string(),
        "install".to_string(),
        "--disable-pip-version-check".to_string(),
        "--no-input".to_string(),
        "--upgrade".to_string(),
        "--force-reinstall".to_string(),
        "--index-url".to_string(),
        profile.install_source.clone(),
    ];
    args.extend(profile.packages.iter().cloned());
    args
}
```

- [ ] **Step 2: Update the existing provision test expectation**

In `src-tauri/src/runtime_profile/provision.rs`, update `pip_install_args_use_profile_install_source_and_exact_packages` expected vector to include:

```rust
"--disable-pip-version-check",
"--no-input",
```

immediately after `"install"`.

- [ ] **Step 3: Harden worker requirements install**

In `src-tauri/src/transcribe.rs`, modify `install_worker_requirements` to include non-interactive pip flags:

```rust
pub(crate) fn install_worker_requirements(python_exe: &Path, requirements_path: &Path) -> Result<(), String> {
    run_python_command(
        python_exe,
        &[
            "-m",
            "pip",
            "install",
            "--disable-pip-version-check",
            "--no-input",
            "--upgrade",
            "--requirement",
            requirements_path
                .to_str()
                .ok_or_else(|| "requirements path is not valid UTF-8".to_string())?,
        ],
        "install worker requirements",
    )
}
```

- [ ] **Step 4: Run targeted tests**

Run:

```powershell
cargo test runtime_profile::provision --lib
```

Working directory: `src-tauri`

Expected: provision tests pass.

- [ ] **Step 5: Commit**

```powershell
git add src-tauri/src/runtime_profile/provision.rs src-tauri/src/transcribe.rs
git commit -m "harden managed runtime pip installs"
```

## Task 6: Update Frontend Type Handling For New Bootstrap Status

**Files:**
- Modify: `src/lib/stores/modelManager.ts`
- Modify: `src/lib/components/Settings.svelte`

- [ ] **Step 1: Add the new status to the TypeScript union**

In `src/lib/stores/modelManager.ts`, add `installing_python_runtime` to the worker runtime bootstrap status union:

```ts
export type WorkerRuntimeBootstrapStatus =
	| 'idle'
	| 'checking'
	| 'installing_python_runtime'
	| 'upgrading_pip'
	| 'installing_cuda_torch'
	| 'installing_worker_requirements'
	| 'falling_back_to_cpu'
	| 'smoke_testing'
	| 'ready'
	| 'error';
```

- [ ] **Step 2: Add user-facing copy for Python runtime preparation**

In `src/lib/components/Settings.svelte`, update the derived runtime bootstrap copy so `installing_python_runtime` renders:

```ts
runtimeBootstrapStatus === 'installing_python_runtime'
	? 'Preparing the bundled Python runtime.'
	: runtimeBootstrapStatus === 'ready'
		? 'Python transcription runtime is ready.'
		: runtimeBootstrapStatus === 'error'
			? ($modelManager.workerRuntimeBootstrap.message ?? 'Python transcription runtime setup failed.')
			: 'Preparing the Python transcription runtime in the background.'
```

Keep the surrounding derived expression style consistent with the file.

- [ ] **Step 3: Run frontend checks**

Run:

```powershell
npm run check
```

Expected: Svelte and TypeScript checks pass.

- [ ] **Step 4: Commit**

```powershell
git add src/lib/stores/modelManager.ts src/lib/components/Settings.svelte
git commit -m "show embedded python runtime bootstrap status"
```

## Task 7: Document Clean-Machine Runtime Flow

**Files:**
- Create: `docs/embedded-python-runtime.md`
- Modify: `docs/runtime-profile-smoke-checks.md`

- [ ] **Step 1: Add runtime documentation**

Create `docs/embedded-python-runtime.md`:

```markdown
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
$env:OK_PYTHON_EXE = "C:\\Python312\\python.exe"
```

Unset it to use the bundled runtime path:

```powershell
Remove-Item Env:OK_PYTHON_EXE
```

Do not set `OK_PYTHON_EXE` when running `npm run tauri:dev:managed-python`; managed mode intentionally ignores it.

## Refreshing The Managed Runtime

Close the app, delete `%LOCALAPPDATA%/com.ok.speech/runtime/python`, and start the app again. The next bootstrap extracts a fresh copy from the bundled resource.

## Verification

Use a machine or VM without Python on `PATH`. Start the published app, open Settings, and wait for runtime setup. The Settings page should report a ready Python transcription runtime without requiring a manual Python install.
```

- [ ] **Step 2: Add clean-machine smoke checks**

Append this section to `docs/runtime-profile-smoke-checks.md`:

```markdown
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
```

- [ ] **Step 3: Commit**

```powershell
git add docs/embedded-python-runtime.md docs/runtime-profile-smoke-checks.md
git commit -m "document embedded python runtime flow"
```

## Task 8: Development Managed Runtime Verification

**Files:**
- No source changes expected

- [ ] **Step 1: Prepare runtime resources**

Run:

```powershell
npm run prepare:python-runtime
```

Expected: checksum verification succeeds and `src-tauri/resources/python` contains the Python zip and `get-pip.py`.

- [ ] **Step 2: Remove the previously extracted managed runtime**

Run:

```powershell
Remove-Item "$env:LOCALAPPDATA\com.ok.speech\runtime\python" -Recurse -Force -ErrorAction SilentlyContinue
```

Expected: the command completes even if the directory was already missing.

- [ ] **Step 3: Run the dev app with managed Python forced**

Run:

```powershell
npm run tauri:dev:managed-python
```

Expected: the app starts in dev mode, `OK_PYTHON_RUNTIME=managed` is set for the Tauri process, and runtime setup does not use `OK_PYTHON_EXE` or `python` on PATH.

- [ ] **Step 4: Verify extracted Python on disk**

After Settings reports runtime progress, check:

```powershell
& "$env:LOCALAPPDATA\com.ok.speech\runtime\python\python.exe" --version
& "$env:LOCALAPPDATA\com.ok.speech\runtime\python\python.exe" -m pip --version
```

Expected: both commands succeed and report Python 3.12.10 plus a working pip installation.

- [ ] **Step 5: Verify runtime profile paths use managed Python**

Load a downloaded model from Settings and inspect the Tauri dev logs.

Expected log shape:

```text
[transcribe] ensure_loaded model_id=... python_exe=...\runtime\python\python.exe worker_script=...\runtime\worker\worker_main.py model_path=...
```

- [ ] **Step 6: Verify profile install and smoke tests on managed Python**

Use the Settings Advanced runtime controls:

```text
Reset to Auto resolves and smoke-tests a profile.
Manual CPU fallback installs CPU torch packages into the managed runtime and smoke-tests successfully.
If an NVIDIA GPU is available, Manual NVIDIA current installs CUDA torch packages into the managed runtime and smoke-tests successfully or reports a clear profile error.
```

- [ ] **Step 7: Verify worker transcription on managed Python**

Run a short recording/transcription with a loaded model.

Expected:

```text
The worker starts from app data, reports Ready, transcribes audio, and records actual execution device in Settings.
```

## Task 9: End-To-End Published Build Verification

**Files:**
- No source changes expected

- [ ] **Step 1: Prepare runtime resources**

Run:

```powershell
npm run prepare:python-runtime
```

Expected: checksum verification succeeds.

- [ ] **Step 2: Run backend tests**

Run:

```powershell
cargo test --lib
```

Working directory: `src-tauri`

Expected: all Rust tests pass.

- [ ] **Step 3: Run frontend checks**

Run:

```powershell
npm run check
```

Expected: Svelte and TypeScript checks pass.

- [ ] **Step 4: Build the published app**

Run:

```powershell
npm run tauri:build
```

Expected: Tauri build completes and includes the Python runtime resources.

- [ ] **Step 5: Verify without system Python**

Run the installer on a clean Windows VM or temporarily ensure `python --version` fails in a normal shell, then start the app.

Expected:

```text
Settings shows runtime setup progressing through Python runtime preparation, pip/profile install, smoke test, and ready.
%LOCALAPPDATA%/com.ok.speech/runtime/python/python.exe exists.
No user-facing prompt asks the user to install Python.
```

- [ ] **Step 6: Verify runtime profile behavior remains intact**

Use `docs/runtime-profile-smoke-checks.md` to verify:

```text
Auto mode activates a compatible profile.
Manual profile selection applies immediately.
CPU fallback still works.
Worker reports actual execution device.
```

- [ ] **Step 7: Final commit if verification changes docs or config**

```powershell
git status --short
git add docs/embedded-python-runtime.md docs/runtime-profile-smoke-checks.md src-tauri/tauri.conf.json package.json src-tauri/Cargo.toml src-tauri/Cargo.lock
git commit -m "verify embedded python runtime packaging"
```

Skip this commit if there are no changes.

## Self-Review

- Spec coverage: The plan ships CPython without PyInstaller, avoids bundling torch, keeps profile-based pip installs, preserves dev override behavior, adds forced managed-runtime development verification, and adds clean-machine verification.
- Completeness scan: No unresolved placeholders remain. SHA-256 values are generated into `runtime-lock.json` by `npm run prepare:python-runtime:lock` and then verified by normal build prep.
- Type consistency: `InstallingPythonRuntime` serializes to `installing_python_runtime`, matching the TypeScript union and Settings copy.
- Scope check: The plan stays focused on managed Python provisioning and testability; it does not change model downloads or runtime profile selection behavior.
