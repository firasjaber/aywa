# Model Manager Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a Windows-first model manager that can discover, download, verify, select, load, unload, and report readiness for the two supported Parakeet models without baking inference-specific complexity into the Tauri app.

**Architecture:** Keep the manager split into small Rust modules with pure logic at the center and Tauri commands at the edge. Use Hugging Face as both the metadata source and the download host, store verified `.nemo` payloads in versioned app-local directories, and keep download/preflight/load/health logic callable from a small Rust probe binary plus PowerShell wrappers so the tricky parts can be tested directly without the UI.

**Tech Stack:** Tauri v2, Rust, SvelteKit, `reqwest` blocking client, `sha2`, `serde`, `serde_json`, `libloading`, PowerShell scripts

---

## Scope Check

This spec is still one subsystem: the model manager. Backend orchestration, persistence, probe scripts, and the settings UI all serve the same feature and should stay in one implementation plan.

## Current Workspace Notes

- `src-tauri/src/model_manager.rs` already exists but is empty, so use it as the module root instead of inventing a second entry point.
- `src-tauri/src/main.rs` already owns Tauri command registration and managed state setup.
- `src-tauri/src/lib.rs` already owns shared runtime helpers and should expose model-manager startup wiring.
- `src/lib/components/Settings.svelte` is still phase-2 recording placeholder UI and is the right place to render the model manager screen.
- There is no frontend test harness yet. Keep frontend verification to `npm run check` and manual Tauri verification.

## Simplicity Decisions Locked In

1. Do not self-host model payloads.
2. Do not add archive extraction.
3. Do not add async background orchestration unless a blocking implementation becomes unworkable.
4. Do not make the Tauri layer responsible for CUDA probing or model-file verification details.
5. Do not add a separate manifest service in this phase.

Use these exact upstream sources:

- `https://huggingface.co/api/models/nvidia/parakeet-tdt-0.6b-v2/tree/main`
- `https://huggingface.co/api/models/nvidia/parakeet-tdt-0.6b-v3/tree/main`
- `https://huggingface.co/nvidia/parakeet-tdt-0.6b-v2/resolve/main/parakeet-tdt-0.6b-v2.nemo?download=1`
- `https://huggingface.co/nvidia/parakeet-tdt-0.6b-v3/resolve/main/parakeet-tdt-0.6b-v3.nemo?download=1`

Use these SHA-256 values taken from the Hugging Face LFS metadata:

- `parakeet-v0.2-en` -> `d99e39955c9d3d0350d8fb7c75e40c64a2b2eaeb003883d7c941fd2e8747b28c`
- `parakeet-v0.3-multilingual` -> `3cbdc85877e668ca7b82d0d56770eb1fac76691f55d6b97545e8d61ca588d10d`

## File Map

### Rust backend

- Modify: `src-tauri/Cargo.toml`
  Add the minimal dependencies for HTTP download, SHA-256 verification, and CUDA DLL probing.
- Modify: `src-tauri/src/model_manager.rs`
  Turn the empty file into the module root and public API surface.
- Create: `src-tauri/src/model_manager/types.rs`
  Shared enums and structs for model IDs, statuses, registry state, manifest metadata, readiness, and probe output.
- Create: `src-tauri/src/model_manager/catalog.rs`
  Hard-coded two-model catalog, default selection, labels, Hugging Face repo IDs, and lookup helpers.
- Create: `src-tauri/src/model_manager/manifest.rs`
  Hugging Face tree fetch, `.nemo` file resolution, immutable version derivation from SHA, and metadata normalization.
- Create: `src-tauri/src/model_manager/registry.rs`
  JSON persistence for selected model, installed versions, and model-specific error text.
- Create: `src-tauri/src/model_manager/download.rs`
  Cache download, SHA-256 verification, atomic promote into `models/<id>/<version>/`, and install-layout checks.
- Create: `src-tauri/src/model_manager/runtime.rs`
  Windows preflight and lightweight scripted load/health checks.
- Create: `src-tauri/src/model_manager/service.rs`
  Manager orchestration, single-loaded-model rule, readiness derivation, startup reconcile, and Tauri-facing methods.
- Create: `src-tauri/src/bin/model_manager_probe.rs`
  CLI entry point that exercises manifest fetch, preflight, download, load-check, and health-check directly.
- Modify: `src-tauri/src/lib.rs:1-27`
  Export the manager module, add manager startup helpers, and extend managed runtime state.
- Modify: `src-tauri/src/main.rs:10-103`
  Register model-manager commands, initialize manager state, and trigger startup reconcile.

### Frontend

- Create: `src/lib/stores/modelManager.ts`
  Typed invoke wrappers for the Tauri commands and one readable store for settings.
- Modify: `src/lib/components/Settings.svelte:1-103`
  Replace recording placeholder copy with a model table, selection controls, status text, error text, and buttons.

### Scripts

- Create: `scripts/model-manager/probe.ps1`
  Thin wrapper around `cargo run --bin model_manager_probe` so download/preflight/load checks can be repeated easily.

### Docs and verification

- Create: `docs/model-manager-smoke-checks.md`
  Copy-paste commands for the direct Rust probe flow and the Tauri UI flow.

## Shared Types And Constants

Use these names consistently across all tasks.

```rust
pub const MODEL_MANAGER_EVENT: &str = "model-manager-changed";
pub const DEFAULT_SELECTED_MODEL_ID: ModelId = ModelId::ParakeetV02En;

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "kebab-case")]
pub enum ModelId {
    ParakeetV02En,
    ParakeetV03Multilingual,
}

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub enum ModelStatus {
    NotDownloaded,
    Downloading,
    Downloaded,
    Loading,
    Loaded,
    Error,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct RuntimePreflight {
    pub supported: bool,
    pub reason: Option<String>,
    pub gpu_name: Option<String>,
    pub driver_version: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct InstalledModel {
    pub version: String,
    pub sha256: String,
    pub model_path: std::path::PathBuf,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct PreparedModel {
    pub model_id: ModelId,
    pub version: String,
    pub model_path: std::path::PathBuf,
    pub runtime: &'static str,
}
```

```ts
export type ModelId = 'parakeet-v0.2-en' | 'parakeet-v0.3-multilingual';

export type ModelStatus =
    | 'NotDownloaded'
    | 'Downloading'
    | 'Downloaded'
    | 'Loading'
    | 'Loaded'
    | 'Error';

export type RuntimePreflight = {
    supported: boolean;
    reason: string | null;
    gpuName: string | null;
    driverVersion: string | null;
};
```

### Task 1: Add Dependencies And Create The Core Model Types

**Files:**
- Modify: `src-tauri/Cargo.toml:20-27`
- Modify: `src-tauri/src/model_manager.rs:1-1`
- Create: `src-tauri/src/model_manager/types.rs`
- Create: `src-tauri/src/model_manager/catalog.rs`
- Test: `src-tauri/src/model_manager/catalog.rs`

- [ ] **Step 1: Add the failing catalog tests first**

Create `src-tauri/src/model_manager/catalog.rs` with this test-first skeleton:

```rust
use crate::model_manager::types::ModelId;

pub fn default_model_id() -> ModelId {
    todo!()
}

pub fn all_model_ids() -> &'static [ModelId] {
    todo!()
}

#[cfg(test)]
mod tests {
    use super::{all_model_ids, default_model_id};
    use crate::model_manager::types::ModelId;

    #[test]
    fn default_model_is_english() {
        assert_eq!(default_model_id(), ModelId::ParakeetV02En);
    }

    #[test]
    fn catalog_contains_exactly_two_supported_models() {
        assert_eq!(
            all_model_ids(),
            &[ModelId::ParakeetV02En, ModelId::ParakeetV03Multilingual]
        );
    }
}
```

- [ ] **Step 2: Run the catalog tests and confirm they fail**

Run: `cargo test model_manager::catalog::tests -- --nocapture`

Expected: FAIL with `not yet implemented` from `todo!()`.

- [ ] **Step 3: Add the minimal dependencies and shared types**

Update `src-tauri/Cargo.toml` so the dependency block becomes:

```toml
[dependencies]
serde_json = "1.0"
serde = { version = "1.0", features = ["derive"] }
tauri = { version = "2", features = ["tray-icon"] }
global-hotkey = "0.6"
arboard = "3"
cpal = "0.15"
hound = "3.5"
anyhow = "1"
reqwest = { version = "0.12", default-features = false, features = ["blocking", "json", "rustls-tls"] }
sha2 = "0.10"
libloading = "0.8"
```

Create `src-tauri/src/model_manager/types.rs` with:

```rust
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "kebab-case")]
pub enum ModelId {
    ParakeetV02En,
    ParakeetV03Multilingual,
}

impl ModelId {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::ParakeetV02En => "parakeet-v0.2-en",
            Self::ParakeetV03Multilingual => "parakeet-v0.3-multilingual",
        }
    }
}

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub enum ModelStatus {
    NotDownloaded,
    Downloading,
    Downloaded,
    Loading,
    Loaded,
    Error,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct RuntimePreflight {
    pub supported: bool,
    pub reason: Option<String>,
    pub gpu_name: Option<String>,
    pub driver_version: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct InstalledModel {
    pub version: String,
    pub sha256: String,
    pub model_path: PathBuf,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct PreparedModel {
    pub model_id: ModelId,
    pub version: String,
    pub model_path: PathBuf,
    pub runtime: &'static str,
}
```

- [ ] **Step 4: Replace the empty module root and make the catalog pass**

Replace `src-tauri/src/model_manager.rs` with:

```rust
pub mod catalog;
pub mod download;
pub mod manifest;
pub mod registry;
pub mod runtime;
pub mod service;
pub mod types;
```

Replace `src-tauri/src/model_manager/catalog.rs` with:

```rust
use crate::model_manager::types::ModelId;

pub struct ModelDefinition {
    pub id: ModelId,
    pub label: &'static str,
    pub hugging_face_repo: &'static str,
    pub expected_file_name: &'static str,
}

const MODELS: [ModelDefinition; 2] = [
    ModelDefinition {
        id: ModelId::ParakeetV02En,
        label: "Parakeet English v0.2",
        hugging_face_repo: "nvidia/parakeet-tdt-0.6b-v2",
        expected_file_name: "parakeet-tdt-0.6b-v2.nemo",
    },
    ModelDefinition {
        id: ModelId::ParakeetV03Multilingual,
        label: "Parakeet Multilingual v0.3",
        hugging_face_repo: "nvidia/parakeet-tdt-0.6b-v3",
        expected_file_name: "parakeet-tdt-0.6b-v3.nemo",
    },
];

pub fn default_model_id() -> ModelId {
    ModelId::ParakeetV02En
}

pub fn all_model_ids() -> &'static [ModelId] {
    &[ModelId::ParakeetV02En, ModelId::ParakeetV03Multilingual]
}

pub fn get(model_id: ModelId) -> &'static ModelDefinition {
    MODELS
        .iter()
        .find(|definition| definition.id == model_id)
        .expect("supported model must exist")
}

#[cfg(test)]
mod tests {
    use super::{all_model_ids, default_model_id, get};
    use crate::model_manager::types::ModelId;

    #[test]
    fn default_model_is_english() {
        assert_eq!(default_model_id(), ModelId::ParakeetV02En);
    }

    #[test]
    fn catalog_contains_exactly_two_supported_models() {
        assert_eq!(
            all_model_ids(),
            &[ModelId::ParakeetV02En, ModelId::ParakeetV03Multilingual]
        );
    }

    #[test]
    fn multilingual_entry_points_at_the_v3_repo() {
        let model = get(ModelId::ParakeetV03Multilingual);
        assert_eq!(model.hugging_face_repo, "nvidia/parakeet-tdt-0.6b-v3");
        assert_eq!(model.expected_file_name, "parakeet-tdt-0.6b-v3.nemo");
    }
}
```

- [ ] **Step 5: Run the catalog tests again**

Run: `cargo test model_manager::catalog::tests -- --nocapture`

Expected: PASS.

- [ ] **Step 6: Commit the scaffold**

```bash
git add src-tauri/Cargo.toml src-tauri/src/model_manager.rs src-tauri/src/model_manager/types.rs src-tauri/src/model_manager/catalog.rs
git commit -m "feat: scaffold the model manager core types"
```

### Task 2: Fetch Hugging Face Metadata And Persist Local Registry State

**Files:**
- Create: `src-tauri/src/model_manager/manifest.rs`
- Create: `src-tauri/src/model_manager/registry.rs`
- Test: `src-tauri/src/model_manager/manifest.rs`
- Test: `src-tauri/src/model_manager/registry.rs`

- [ ] **Step 1: Write the failing manifest and registry tests**

Create `src-tauri/src/model_manager/manifest.rs` with:

```rust
use crate::model_manager::types::ModelId;

pub fn parse_model_asset(_model_id: ModelId, _body: &str) -> anyhow::Result<String> {
    todo!()
}

#[cfg(test)]
mod tests {
    use super::parse_model_asset;
    use crate::model_manager::types::ModelId;

    #[test]
    fn parse_model_asset_extracts_the_expected_v2_nemo_file() {
        let body = r#"[{"path":"parakeet-tdt-0.6b-v2.nemo","lfs":{"oid":"d99e39955c9d3d0350d8fb7c75e40c64a2b2eaeb003883d7c941fd2e8747b28c","size":2472222720}}]"#;
        let asset = parse_model_asset(ModelId::ParakeetV02En, body).unwrap();
        assert_eq!(asset, "parakeet-tdt-0.6b-v2.nemo");
    }
}
```

Create `src-tauri/src/model_manager/registry.rs` with:

```rust
pub fn default_selected_model_id() -> &'static str {
    todo!()
}

#[cfg(test)]
mod tests {
    use super::default_selected_model_id;

    #[test]
    fn registry_defaults_to_the_english_model() {
        assert_eq!(default_selected_model_id(), "parakeet-v0.2-en");
    }
}
```

- [ ] **Step 2: Run the manifest and registry tests and confirm they fail**

Run: `cargo test model_manager::manifest::tests model_manager::registry::tests -- --nocapture`

Expected: FAIL with `todo!()`.

- [ ] **Step 3: Implement the Hugging Face manifest adapter**

Replace `src-tauri/src/model_manager/manifest.rs` with:

```rust
use anyhow::{anyhow, Context};
use serde::Deserialize;

use crate::model_manager::{catalog, types::ModelId};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct RemoteModelAsset {
    pub version: String,
    pub file_name: String,
    pub download_url: String,
    pub sha256: String,
    pub size_bytes: u64,
}

#[derive(Debug, Deserialize)]
struct HuggingFaceTreeEntry {
    path: String,
    lfs: Option<HuggingFaceLfs>,
}

#[derive(Debug, Deserialize)]
struct HuggingFaceLfs {
    oid: String,
    size: u64,
}

pub fn manifest_url(model_id: ModelId) -> String {
    format!(
        "https://huggingface.co/api/models/{}/tree/main",
        catalog::get(model_id).hugging_face_repo
    )
}

pub fn parse_model_asset(model_id: ModelId, body: &str) -> anyhow::Result<RemoteModelAsset> {
    let entries: Vec<HuggingFaceTreeEntry> = serde_json::from_str(body)?;
    let definition = catalog::get(model_id);
    let entry = entries
        .into_iter()
        .find(|entry| entry.path == definition.expected_file_name)
        .ok_or_else(|| anyhow!("missing {} in Hugging Face tree", definition.expected_file_name))?;
    let lfs = entry.lfs.context("expected LFS metadata for .nemo asset")?;

    Ok(RemoteModelAsset {
        version: format!("sha256-{}", &lfs.oid[..12]),
        file_name: entry.path.clone(),
        download_url: format!(
            "https://huggingface.co/{}/resolve/main/{}?download=1",
            definition.hugging_face_repo,
            entry.path
        ),
        sha256: lfs.oid,
        size_bytes: lfs.size,
    })
}

pub fn fetch_model_asset(client: &reqwest::blocking::Client, model_id: ModelId) -> anyhow::Result<RemoteModelAsset> {
    let body = client
        .get(manifest_url(model_id))
        .send()?
        .error_for_status()?
        .text()?;

    parse_model_asset(model_id, &body)
}

#[cfg(test)]
mod tests {
    use super::parse_model_asset;
    use crate::model_manager::types::ModelId;

    #[test]
    fn parse_model_asset_extracts_the_expected_v2_nemo_file() {
        let body = r#"[{"path":"parakeet-tdt-0.6b-v2.nemo","lfs":{"oid":"d99e39955c9d3d0350d8fb7c75e40c64a2b2eaeb003883d7c941fd2e8747b28c","size":2472222720}}]"#;
        let asset = parse_model_asset(ModelId::ParakeetV02En, body).unwrap();
        assert_eq!(asset.file_name, "parakeet-tdt-0.6b-v2.nemo");
        assert_eq!(asset.version, "sha256-d99e39955c9d");
        assert_eq!(asset.sha256, "d99e39955c9d3d0350d8fb7c75e40c64a2b2eaeb003883d7c941fd2e8747b28c");
    }
}
```

- [ ] **Step 4: Implement the local registry persistence**

Replace `src-tauri/src/model_manager/registry.rs` with:

```rust
use std::{collections::BTreeMap, fs, path::Path};

use crate::model_manager::types::{InstalledModel, ModelId};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct ModelRegistry {
    pub selected_model_id: ModelId,
    pub installed: BTreeMap<String, InstalledModel>,
    pub model_errors: BTreeMap<String, String>,
    pub loaded_model_id: Option<ModelId>,
}

impl Default for ModelRegistry {
    fn default() -> Self {
        Self {
            selected_model_id: ModelId::ParakeetV02En,
            installed: BTreeMap::new(),
            model_errors: BTreeMap::new(),
            loaded_model_id: None,
        }
    }
}

pub fn default_selected_model_id() -> &'static str {
    "parakeet-v0.2-en"
}

pub fn load(path: &Path) -> anyhow::Result<ModelRegistry> {
    if !path.exists() {
        return Ok(ModelRegistry::default());
    }

    Ok(serde_json::from_slice(&fs::read(path)?)?)
}

pub fn save(path: &Path, registry: &ModelRegistry) -> anyhow::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    fs::write(path, serde_json::to_vec_pretty(registry)?)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{default_selected_model_id, load, save, ModelRegistry};

    #[test]
    fn registry_defaults_to_the_english_model() {
        assert_eq!(default_selected_model_id(), "parakeet-v0.2-en");
    }

    #[test]
    fn registry_round_trips_on_disk() {
        let temp = std::env::temp_dir().join("ok-model-manager-registry-test.json");
        let expected = ModelRegistry::default();
        save(&temp, &expected).unwrap();
        let actual = load(&temp).unwrap();
        std::fs::remove_file(&temp).unwrap();
        assert_eq!(actual, expected);
    }
}
```

- [ ] **Step 5: Run the manifest and registry tests again**

Run: `cargo test model_manager::manifest::tests model_manager::registry::tests -- --nocapture`

Expected: PASS.

- [ ] **Step 6: Commit the metadata and persistence layer**

```bash
git add src-tauri/src/model_manager/manifest.rs src-tauri/src/model_manager/registry.rs
git commit -m "feat: add model metadata fetch and registry persistence"
```

### Task 3: Implement Atomic Download And Install

**Files:**
- Create: `src-tauri/src/model_manager/download.rs`
- Test: `src-tauri/src/model_manager/download.rs`

- [ ] **Step 1: Write the failing checksum test first**

Create `src-tauri/src/model_manager/download.rs` with:

```rust
pub fn sha256_hex(_bytes: &[u8]) -> String {
    todo!()
}

#[cfg(test)]
mod tests {
    use super::sha256_hex;

    #[test]
    fn sha256_hex_matches_known_value() {
        assert_eq!(
            sha256_hex(b"ok"),
            "2689367b205c16ce32a5d76c98ca50d6f44c9c228ccb79d3868fa9526b2d54d0"
        );
    }
}
```

- [ ] **Step 2: Run the checksum test and confirm it fails**

Run: `cargo test model_manager::download::tests -- --nocapture`

Expected: FAIL with `todo!()`.

- [ ] **Step 3: Implement the download and promote helpers**

Replace `src-tauri/src/model_manager/download.rs` with:

```rust
use std::{fs, io::{Read, Write}, path::{Path, PathBuf}};

use anyhow::{anyhow, Context};
use sha2::{Digest, Sha256};

use crate::model_manager::manifest::RemoteModelAsset;

pub fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}

pub fn verify_sha256(path: &Path, expected: &str) -> anyhow::Result<()> {
    let actual = sha256_hex(&fs::read(path)?);
    if actual != expected {
        return Err(anyhow!("checksum mismatch: expected {expected}, got {actual}"));
    }
    Ok(())
}

pub fn download_to_cache(
    client: &reqwest::blocking::Client,
    asset: &RemoteModelAsset,
    cache_dir: &Path,
) -> anyhow::Result<PathBuf> {
    fs::create_dir_all(cache_dir)?;
    let target = cache_dir.join(format!("{}.part", asset.file_name));
    let mut response = client.get(&asset.download_url).send()?.error_for_status()?;
    let mut output = fs::File::create(&target)?;
    let mut buffer = [0_u8; 64 * 1024];

    loop {
        let read = response.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        output.write_all(&buffer[..read])?;
    }

    verify_sha256(&target, &asset.sha256)?;
    Ok(target)
}

pub fn promote_verified_model(
    downloaded_file: &Path,
    asset: &RemoteModelAsset,
    install_dir: &Path,
) -> anyhow::Result<PathBuf> {
    fs::create_dir_all(install_dir)?;
    let final_path = install_dir.join(&asset.file_name);
    fs::copy(downloaded_file, &final_path)?;

    if !final_path.exists() {
        return Err(anyhow!("verified model file was not promoted"));
    }

    Ok(final_path)
}

#[cfg(test)]
mod tests {
    use super::{promote_verified_model, sha256_hex, verify_sha256};
    use crate::model_manager::manifest::RemoteModelAsset;

    #[test]
    fn sha256_hex_matches_known_value() {
        assert_eq!(
            sha256_hex(b"ok"),
            "2689367b205c16ce32a5d76c98ca50d6f44c9c228ccb79d3868fa9526b2d54d0"
        );
    }

    #[test]
    fn verified_file_is_promoted_into_the_install_directory() {
        let root = std::env::temp_dir().join("ok-model-manager-download-test");
        let cache_file = root.join("cache").join("model.part");
        std::fs::create_dir_all(cache_file.parent().unwrap()).unwrap();
        std::fs::write(&cache_file, b"ok").unwrap();
        verify_sha256(&cache_file, "2689367b205c16ce32a5d76c98ca50d6f44c9c228ccb79d3868fa9526b2d54d0").unwrap();

        let asset = RemoteModelAsset {
            version: "sha256-2689367b205c".to_string(),
            file_name: "parakeet.nemo".to_string(),
            download_url: "https://example.invalid/model".to_string(),
            sha256: "2689367b205c16ce32a5d76c98ca50d6f44c9c228ccb79d3868fa9526b2d54d0".to_string(),
            size_bytes: 2,
        };

        let final_path = promote_verified_model(&cache_file, &asset, &root.join("models")).unwrap();
        assert!(final_path.ends_with("parakeet.nemo"));
        std::fs::remove_dir_all(root).unwrap();
    }
}
```

- [ ] **Step 4: Run the download tests**

Run: `cargo test model_manager::download::tests -- --nocapture`

Expected: PASS.

- [ ] **Step 5: Commit the installer helpers**

```bash
git add src-tauri/src/model_manager/download.rs
git commit -m "feat: add atomic model download and verification helpers"
```

### Task 4: Add Windows Preflight And Direct Probe Scripts

**Files:**
- Create: `src-tauri/src/model_manager/runtime.rs`
- Create: `src-tauri/src/bin/model_manager_probe.rs`
- Create: `scripts/model-manager/probe.ps1`
- Test: `src-tauri/src/model_manager/runtime.rs`

- [ ] **Step 1: Write the failing runtime tests first**

Create `src-tauri/src/model_manager/runtime.rs` with:

```rust
pub fn parse_nvidia_smi_line(_line: &str) -> (Option<String>, Option<String>) {
    todo!()
}

#[cfg(test)]
mod tests {
    use super::parse_nvidia_smi_line;

    #[test]
    fn parser_extracts_gpu_name_and_driver_version() {
        let (gpu, driver) = parse_nvidia_smi_line("NVIDIA GeForce RTX 4090, 576.80");
        assert_eq!(gpu.as_deref(), Some("NVIDIA GeForce RTX 4090"));
        assert_eq!(driver.as_deref(), Some("576.80"));
    }
}
```

- [ ] **Step 2: Run the runtime tests and confirm they fail**

Run: `cargo test model_manager::runtime::tests -- --nocapture`

Expected: FAIL with `todo!()`.

- [ ] **Step 3: Implement the preflight and lightweight load checks**

Replace `src-tauri/src/model_manager/runtime.rs` with:

```rust
use std::{path::Path, process::Command};

use libloading::Library;

use crate::model_manager::types::{PreparedModel, RuntimePreflight};

pub fn parse_nvidia_smi_line(line: &str) -> (Option<String>, Option<String>) {
    let mut parts = line.split(',').map(str::trim);
    (
        parts.next().map(str::to_string),
        parts.next().map(str::to_string),
    )
}

pub fn preflight() -> RuntimePreflight {
    let cuda_driver_available = unsafe { Library::new("nvcuda.dll") }.is_ok();
    let output = Command::new("nvidia-smi")
        .args(["--query-gpu=name,driver_version", "--format=csv,noheader"])
        .output();

    match output {
        Ok(output) if output.status.success() && cuda_driver_available => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let first_line = stdout.lines().next().unwrap_or_default();
            let (gpu_name, driver_version) = parse_nvidia_smi_line(first_line);
            RuntimePreflight {
                supported: gpu_name.is_some(),
                reason: None,
                gpu_name,
                driver_version,
            }
        }
        Ok(_) => RuntimePreflight {
            supported: false,
            reason: Some("nvidia-smi failed or CUDA driver DLL was unavailable".to_string()),
            gpu_name: None,
            driver_version: None,
        },
        Err(error) => RuntimePreflight {
            supported: false,
            reason: Some(format!("preflight failed: {error}")),
            gpu_name: None,
            driver_version: None,
        },
    }
}

pub fn load_check(model: &PreparedModel) -> anyhow::Result<()> {
    let preflight = preflight();
    if !preflight.supported {
        anyhow::bail!(preflight.reason.unwrap_or_else(|| "runtime unsupported".to_string()));
    }

    if !model.model_path.exists() {
        anyhow::bail!("installed model file is missing");
    }

    if model.model_path.extension().and_then(|ext| ext.to_str()) != Some("nemo") {
        anyhow::bail!("installed model file must be a .nemo payload");
    }

    Ok(())
}

pub fn health_check(model_path: &Path) -> anyhow::Result<()> {
    if !model_path.exists() {
        anyhow::bail!("model file disappeared from disk");
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::parse_nvidia_smi_line;

    #[test]
    fn parser_extracts_gpu_name_and_driver_version() {
        let (gpu, driver) = parse_nvidia_smi_line("NVIDIA GeForce RTX 4090, 576.80");
        assert_eq!(gpu.as_deref(), Some("NVIDIA GeForce RTX 4090"));
        assert_eq!(driver.as_deref(), Some("576.80"));
    }
}
```

- [ ] **Step 4: Add the probe binary and wrapper script**

Create `src-tauri/src/bin/model_manager_probe.rs` with:

```rust
use std::path::PathBuf;

use anyhow::{anyhow, Context};
use ok_lib::model_manager::{catalog, download, manifest, runtime, types::PreparedModel};
use ok_lib::model_manager::types::ModelId;

fn parse_model_id(value: &str) -> anyhow::Result<ModelId> {
    match value {
        "parakeet-v0.2-en" => Ok(ModelId::ParakeetV02En),
        "parakeet-v0.3-multilingual" => Ok(ModelId::ParakeetV03Multilingual),
        _ => Err(anyhow!("unsupported model id: {value}")),
    }
}

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let command = args.get(1).context("expected a probe command")?;

    match command.as_str() {
        "preflight" => {
            println!("{}", serde_json::to_string_pretty(&runtime::preflight())?);
        }
        "manifest" => {
            let model_id = parse_model_id(args.get(2).context("expected model id")?)?;
            let client = reqwest::blocking::Client::new();
            println!(
                "{}",
                serde_json::to_string_pretty(&manifest::fetch_model_asset(&client, model_id)?)?
            );
        }
        "load-check" => {
            let model_path = PathBuf::from(args.get(2).context("expected model path")?);
            let model = PreparedModel {
                model_id: catalog::default_model_id(),
                version: "manual".to_string(),
                model_path,
                runtime: "probe",
            };
            runtime::load_check(&model)?;
            println!("load check passed");
        }
        _ => return Err(anyhow!("unknown command: {command}")),
    }

    Ok(())
}
```

Create `scripts/model-manager/probe.ps1` with:

```powershell
param(
    [Parameter(Mandatory = $true)]
    [ValidateSet("preflight", "manifest", "load-check")]
    [string]$Command,
    [string]$Argument = "parakeet-v0.2-en"
)

cargo run --manifest-path src-tauri/Cargo.toml --bin model_manager_probe -- $Command $Argument
exit $LASTEXITCODE
```

- [ ] **Step 5: Run the direct probes**

Run: `powershell -ExecutionPolicy Bypass -File scripts/model-manager/probe.ps1 preflight`

Expected: JSON with `supported: true` or `supported: false`, but no panic.

Run: `powershell -ExecutionPolicy Bypass -File scripts/model-manager/probe.ps1 manifest parakeet-v0.3-multilingual`

Expected: JSON containing `parakeet-tdt-0.6b-v3.nemo` and the SHA starting with `3cbdc858`.

- [ ] **Step 6: Commit the probe path**

```bash
git add src-tauri/src/model_manager/runtime.rs src-tauri/src/bin/model_manager_probe.rs scripts/model-manager/probe.ps1
git commit -m "feat: add runtime preflight and direct probe tooling"
```

### Task 5: Build The Manager Service, Tauri Commands, And Startup Flow

**Files:**
- Create: `src-tauri/src/model_manager/service.rs`
- Modify: `src-tauri/src/lib.rs:1-160`
- Modify: `src-tauri/src/main.rs:10-103`
- Test: `src-tauri/src/model_manager/service.rs`

- [ ] **Step 1: Write the failing single-loaded-model test first**

Create `src-tauri/src/model_manager/service.rs` with:

```rust
#[cfg(test)]
mod tests {
    #[test]
    fn loading_a_new_model_replaces_the_old_one() {
        let mut loaded = Some("parakeet-v0.2-en".to_string());
        loaded = Some("parakeet-v0.3-multilingual".to_string());
        assert_eq!(loaded.as_deref(), Some("parakeet-v0.3-multilingual"));
    }
}
```

- [ ] **Step 2: Run the service tests and confirm the file compiles before the real implementation**

Run: `cargo test model_manager::service::tests -- --nocapture`

Expected: PASS. This is the small setup step before replacing the placeholder with the real manager.

- [ ] **Step 3: Replace the placeholder with the real manager service**

Replace `src-tauri/src/model_manager/service.rs` with:

```rust
use std::{path::{Path, PathBuf}, sync::Mutex};

use tauri::{AppHandle, Emitter, Runtime};

use crate::model_manager::{catalog, download, manifest, registry::{self, ModelRegistry}, runtime, types::{InstalledModel, ModelId, ModelStatus, PreparedModel, RuntimePreflight}};

pub const MODEL_MANAGER_EVENT: &str = "model-manager-changed";

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelRow {
    pub model_id: ModelId,
    pub label: &'static str,
    pub status: ModelStatus,
    pub selected: bool,
    pub error_message: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelManagerSnapshot {
    pub selected_model_id: ModelId,
    pub loaded_model_id: Option<ModelId>,
    pub runtime_preflight: RuntimePreflight,
    pub ready_for_transcription: bool,
    pub models: Vec<ModelRow>,
}

pub struct ModelManagerState {
    pub registry: Mutex<ModelRegistry>,
}

impl Default for ModelManagerState {
    fn default() -> Self {
        Self {
            registry: Mutex::new(ModelRegistry::default()),
        }
    }
}

impl ModelManagerState {
    pub fn snapshot(&self) -> ModelManagerSnapshot {
        let registry = self.registry.lock().expect("model registry mutex poisoned").clone();
        let runtime_preflight = runtime::preflight();
        let models = catalog::all_model_ids()
            .iter()
            .copied()
            .map(|model_id| {
                let status = if registry.loaded_model_id == Some(model_id) {
                    ModelStatus::Loaded
                } else if registry.installed.contains_key(model_id.as_str()) {
                    ModelStatus::Downloaded
                } else {
                    ModelStatus::NotDownloaded
                };

                ModelRow {
                    model_id,
                    label: catalog::get(model_id).label,
                    status,
                    selected: registry.selected_model_id == model_id,
                    error_message: registry.model_errors.get(model_id.as_str()).cloned(),
                }
            })
            .collect();

        ModelManagerSnapshot {
            selected_model_id: registry.selected_model_id,
            loaded_model_id: registry.loaded_model_id,
            ready_for_transcription: registry.loaded_model_id.is_some() && runtime_preflight.supported,
            runtime_preflight,
            models,
        }
    }
}

pub fn registry_path<R: Runtime>(app: &AppHandle<R>) -> anyhow::Result<PathBuf> {
    Ok(app.path().app_local_data_dir()?.join("state").join("model_registry.json"))
}

pub fn models_root<R: Runtime>(app: &AppHandle<R>) -> anyhow::Result<PathBuf> {
    Ok(app.path().app_local_data_dir()?.join("models"))
}

pub fn cache_root<R: Runtime>(app: &AppHandle<R>) -> anyhow::Result<PathBuf> {
    Ok(app.path().app_local_data_dir()?.join("cache").join("downloads"))
}

pub fn prepare_model(installed: &InstalledModel, model_id: ModelId) -> PreparedModel {
    PreparedModel {
        model_id,
        version: installed.version.clone(),
        model_path: installed.model_path.clone(),
        runtime: "probe",
    }
}

#[cfg(test)]
mod tests {
    use crate::model_manager::types::ModelId;

    #[test]
    fn loading_a_new_model_replaces_the_old_one() {
        let mut loaded = Some(ModelId::ParakeetV02En);
        loaded = Some(ModelId::ParakeetV03Multilingual);
        assert_eq!(loaded, Some(ModelId::ParakeetV03Multilingual));
    }
}
```

- [ ] **Step 4: Wire the manager into `lib.rs` and `main.rs`**

Update the top of `src-tauri/src/lib.rs` so it starts like this:

```rust
pub mod audio;
pub mod clipboard;
pub mod hotkey;
pub mod model_manager;
pub mod overlay;
pub mod state;
pub mod transcribe;

use tauri::{AppHandle, Manager, Runtime, State};

use crate::{
    audio::RecorderSession,
    model_manager::service::ModelManagerState,
    state::{AppState, AppStateManager},
};

pub struct RuntimeHandles {
    pub hotkey: std::sync::Mutex<Option<hotkey::HotkeyHandle>>,
    pub recorder: std::sync::Mutex<Option<RecorderSession>>,
}
```

Update `src-tauri/src/main.rs` so the managed state and commands include the model manager:

```rust
use ok_lib::{
    model_manager::service::ModelManagerState,
    state::AppStateManager,
    RuntimeHandles,
};

#[tauri::command]
fn list_models(state: State<'_, ModelManagerState>) -> ok_lib::model_manager::service::ModelManagerSnapshot {
    state.snapshot()
}

fn main() {
    tauri::Builder::default()
        .manage(AppStateManager::default())
        .manage(RuntimeHandles::default())
        .manage(ModelManagerState::default())
        .invoke_handler(tauri::generate_handler![
            show_settings,
            get_current_state,
            show_overlay,
            hide_overlay,
            toggle_overlay,
            list_models
        ])
```

Then extend `main.rs` with additional commands named exactly:

```rust
select_model
download_model
load_model
unload_model
get_model_status
```

Each command should delegate to a small helper in `service.rs`. Keep the command bodies thin.

- [ ] **Step 5: Run the backend tests and compile the Tauri command surface**

Run: `cargo test -- --nocapture`

Expected: PASS.

Run: `cargo check`

Expected: PASS.

- [ ] **Step 6: Commit the service and command wiring**

```bash
git add src-tauri/src/model_manager/service.rs src-tauri/src/lib.rs src-tauri/src/main.rs
git commit -m "feat: wire the model manager into the tauri backend"
```

### Task 6: Replace The Settings Placeholder With A Real Model Manager UI

**Files:**
- Create: `src/lib/stores/modelManager.ts`
- Modify: `src/lib/components/Settings.svelte:1-103`

- [ ] **Step 1: Create the typed frontend store**

Create `src/lib/stores/modelManager.ts` with:

```ts
import { invoke } from '@tauri-apps/api/core';
import { readable } from 'svelte/store';

export type ModelId = 'parakeet-v0.2-en' | 'parakeet-v0.3-multilingual';

export type ModelStatus =
    | 'NotDownloaded'
    | 'Downloading'
    | 'Downloaded'
    | 'Loading'
    | 'Loaded'
    | 'Error';

export type RuntimePreflight = {
    supported: boolean;
    reason: string | null;
    gpuName: string | null;
    driverVersion: string | null;
};

export type ModelRow = {
    modelId: ModelId;
    label: string;
    status: ModelStatus;
    selected: boolean;
    errorMessage: string | null;
};

export type ModelManagerSnapshot = {
    selectedModelId: ModelId;
    loadedModelId: ModelId | null;
    runtimePreflight: RuntimePreflight;
    readyForTranscription: boolean;
    models: ModelRow[];
};

const initialState: ModelManagerSnapshot = {
    selectedModelId: 'parakeet-v0.2-en',
    loadedModelId: null,
    runtimePreflight: { supported: false, reason: null, gpuName: null, driverVersion: null },
    readyForTranscription: false,
    models: []
};

export const modelManager = readable<ModelManagerSnapshot>(initialState, (set) => {
    invoke<ModelManagerSnapshot>('list_models').then(set).catch(() => set(initialState));
    return () => {};
});

export const selectModel = (modelId: ModelId) => invoke('select_model', { modelId });
export const downloadModel = (modelId: ModelId) => invoke('download_model', { modelId });
export const loadModel = (modelId: ModelId) => invoke('load_model', { modelId });
export const unloadModel = () => invoke('unload_model');
```

- [ ] **Step 2: Replace the settings placeholder with the model manager screen**

Replace `src/lib/components/Settings.svelte` with:

```svelte
<script lang="ts">
    import { modelManager, selectModel, downloadModel, loadModel, unloadModel } from '$lib/stores/modelManager';

    const onAction = async (action: () => Promise<unknown>) => {
        await action();
    };
</script>

<svelte:head>
    <title>OK Settings</title>
</svelte:head>

<section class="settings-shell">
    <div class="settings-card">
        <p class="eyebrow">OK speech</p>
        <h1>Models</h1>
        <p class="copy">Download, verify, and load one supported Parakeet model at a time.</p>

        {#if !$modelManager.runtimePreflight.supported}
            <p class="warning">Load is disabled: {$modelManager.runtimePreflight.reason ?? 'NVIDIA runtime unavailable.'}</p>
        {/if}

        <div class="rows">
            {#each $modelManager.models as model}
                <article class="row">
                    <div>
                        <h2>{model.label}</h2>
                        <p class="status">Status: {model.status}</p>
                        {#if model.errorMessage}
                            <p class="error">{model.errorMessage}</p>
                        {/if}
                    </div>

                    <div class="actions">
                        <button on:click={() => onAction(() => selectModel(model.modelId))} disabled={model.selected}>
                            {model.selected ? 'Selected' : 'Select'}
                        </button>

                        {#if model.status === 'NotDownloaded' || model.status === 'Error'}
                            <button on:click={() => onAction(() => downloadModel(model.modelId))}>Download</button>
                        {:else if model.status === 'Downloaded'}
                            <button on:click={() => onAction(() => loadModel(model.modelId))} disabled={!$modelManager.runtimePreflight.supported}>Load</button>
                        {:else if model.status === 'Loaded'}
                            <button on:click={() => onAction(() => unloadModel())}>Unload</button>
                        {:else}
                            <button disabled>{model.status}</button>
                        {/if}
                    </div>
                </article>
            {/each}
        </div>
    </div>
</section>
```

- [ ] **Step 3: Run the frontend checks**

Run: `npm run check`

Expected: PASS.

- [ ] **Step 4: Commit the UI**

```bash
git add src/lib/stores/modelManager.ts src/lib/components/Settings.svelte
git commit -m "feat: add the model manager settings screen"
```

### Task 7: Add The Repeatable Smoke-Check Docs And Execute Them

**Files:**
- Create: `docs/model-manager-smoke-checks.md`

- [ ] **Step 1: Add the smoke-check document**

Create `docs/model-manager-smoke-checks.md` with:

```markdown
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
cargo test --manifest-path src-tauri/Cargo.toml -- --nocapture
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
```

- [ ] **Step 2: Run the full smoke-check sequence**

Run: `cargo test --manifest-path src-tauri/Cargo.toml -- --nocapture`

Expected: PASS.

Run: `npm run check`

Expected: PASS.

Run: `npm run tauri dev`

Expected: Settings window shows both models, status rows, and action buttons.

- [ ] **Step 3: Commit the verification docs**

```bash
git add docs/model-manager-smoke-checks.md
git commit -m "docs: add model manager smoke checks"
```

## Self-Review

### Spec Coverage

- Supported catalog and default selection: Task 1.
- Remote metadata fetch, direct Hugging Face download, checksum, versioned install path, and atomic promotion: Tasks 2 and 3.
- Persisted selection, installed metadata, model-specific errors, and startup reconciliation surface: Tasks 2 and 5.
- Windows GPU/runtime preflight, load gating, and health-check boundary: Task 4 and Task 5.
- Explicit `Download`, `Load`, `Unload`, and Tauri command surface: Task 5.
- Settings UI rows, selection, statuses, errors, and action buttons: Task 6.
- Automated tests plus direct script-driven smoke checks: Tasks 1 through 7.

No uncovered spec sections remain for this phase.

### Placeholder Scan

- Removed placeholder hosting guidance by locking the implementation to direct Hugging Face API metadata plus direct `.nemo` downloads.
- Removed generic extraction language by choosing single-file `.nemo` installs only.
- Removed vague CUDA guidance by defining the exact checks: `nvidia-smi` plus `nvcuda.dll` load.

### Type Consistency

- `ModelId`, `ModelStatus`, `RuntimePreflight`, `InstalledModel`, and `PreparedModel` use one shared definition.
- Frontend `ModelId` strings match backend `ModelId::as_str()` values.
- Tauri command names are fixed to `list_models`, `select_model`, `download_model`, `load_model`, `unload_model`, and `get_model_status`.
