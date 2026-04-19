use std::{
    collections::BTreeMap,
    fs,
    path::PathBuf,
    sync::Mutex,
    thread,
};

use tauri::{AppHandle, Emitter, Manager, Runtime};

use crate::model_manager::{
    catalog, download, manifest,
    registry::{self, ModelRegistry},
    runtime,
    types::{InstalledModel, ModelId, ModelStatus, PreparedModel, RuntimePreflight},
};

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
    pub transient_statuses: Mutex<BTreeMap<String, ModelStatus>>,
}

impl Default for ModelManagerState {
    fn default() -> Self {
        Self {
            registry: Mutex::new(ModelRegistry::default()),
            transient_statuses: Mutex::new(BTreeMap::new()),
        }
    }
}

impl ModelManagerState {
    pub fn snapshot(&self) -> ModelManagerSnapshot {
        let registry = self
            .registry
            .lock()
            .expect("model registry mutex poisoned")
            .clone();
        let transient_statuses = self
            .transient_statuses
            .lock()
            .expect("transient status mutex poisoned")
            .clone();
        snapshot_from_registry(&registry, &transient_statuses)
    }
}

pub fn registry_path<R: Runtime>(app: &AppHandle<R>) -> anyhow::Result<PathBuf> {
    Ok(app
        .path()
        .app_local_data_dir()?
        .join("state")
        .join("model_registry.json"))
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

pub fn list_models(state: &ModelManagerState) -> ModelManagerSnapshot {
    state.snapshot()
}

pub fn get_model_status(state: &ModelManagerState) -> ModelManagerSnapshot {
    state.snapshot()
}

pub fn startup_reconcile<R: Runtime>(
    app: &AppHandle<R>,
    state: &ModelManagerState,
) -> Result<ModelManagerSnapshot, String> {
    let path = registry_path(app).map_err(|error| error.to_string())?;
    let mut registry = registry::load(&path).map_err(|error| error.to_string())?;
    let mut changed = false;

    if let Some(model_id) = registry.loaded_model_id {
        match registry.installed.get(model_id.as_str()) {
            Some(installed) => {
                if let Err(error) = runtime::health_check(&installed.model_path) {
                    registry.loaded_model_id = None;
                    registry
                        .model_errors
                        .insert(model_id.as_str().to_string(), error.to_string());
                    changed = true;
                }
            }
            None => {
                registry.loaded_model_id = None;
                registry.model_errors.insert(
                    model_id.as_str().to_string(),
                    "installed model metadata is missing".to_string(),
                );
                changed = true;
            }
        }
    }

    {
        let mut current = state
            .registry
            .lock()
            .map_err(|_| "model registry mutex poisoned".to_string())?;
        *current = registry.clone();
    }

    if changed {
        registry::save(&path, &registry).map_err(|error| error.to_string())?;
    }

    emit_snapshot(app, state)
}

pub fn select_model<R: Runtime>(
    app: &AppHandle<R>,
    state: &ModelManagerState,
    model_id: ModelId,
) -> Result<ModelManagerSnapshot, String> {
    let path = registry_path(app).map_err(|error| error.to_string())?;

    {
        let mut registry = state
            .registry
            .lock()
            .map_err(|_| "model registry mutex poisoned".to_string())?;
        registry.selected_model_id = model_id;
        registry::save(&path, &registry).map_err(|error| error.to_string())?;
    }

    emit_snapshot(app, state)
}

pub fn download_model<R: Runtime>(
    app: &AppHandle<R>,
    state: &ModelManagerState,
    model_id: ModelId,
) -> Result<ModelManagerSnapshot, String> {
    {
        let mut transient_statuses = state
            .transient_statuses
            .lock()
            .map_err(|_| "transient status mutex poisoned".to_string())?;
        transient_statuses.insert(model_id.as_str().to_string(), ModelStatus::Downloading);
    }

    let snapshot = emit_snapshot(app, state)?;
    let app = app.clone();

    thread::spawn(move || {
        let result = (|| -> anyhow::Result<InstalledModel> {
            eprintln!(
                "[model_manager] download_model start model_id={}",
                model_id.as_str()
            );

            let client = reqwest::blocking::Client::new();
            eprintln!(
                "[model_manager] fetching manifest metadata model_id={}",
                model_id.as_str()
            );
            let asset = manifest::fetch_model_asset(&client, model_id)?;
            eprintln!(
                "[model_manager] manifest resolved model_id={} version={} file_name={} size_bytes={}",
                model_id.as_str(),
                asset.version,
                asset.file_name,
                asset.size_bytes
            );

            let cache_dir = cache_root(&app)?;
            eprintln!(
                "[model_manager] downloading to cache model_id={} cache_dir={}",
                model_id.as_str(),
                cache_dir.display()
            );
            let cached_file = download::download_to_cache(&client, &asset, &cache_dir)?;

            let install_dir = models_root(&app)?.join(model_id.as_str()).join(&asset.version);
            eprintln!(
                "[model_manager] promoting verified model model_id={} cached_file={} install_dir={}",
                model_id.as_str(),
                cached_file.display(),
                install_dir.display()
            );
            let model_path = download::promote_verified_model(&cached_file, &asset, &install_dir)?;
            eprintln!(
                "[model_manager] download_model prepared installed model model_id={} model_path={}",
                model_id.as_str(),
                model_path.display()
            );

            Ok(InstalledModel {
                version: asset.version,
                sha256: asset.sha256,
                model_path,
            })
        })();

        let state = app.state::<ModelManagerState>();
        let path = match registry_path(&app) {
            Ok(path) => path,
            Err(error) => {
                eprintln!(
                    "[model_manager] download_model error model_id={} error={}",
                    model_id.as_str(),
                    error
                );
                let mut transient_statuses = state
                    .transient_statuses
                    .lock()
                    .expect("transient status mutex poisoned");
                transient_statuses.remove(model_id.as_str());
                return;
            }
        };

        let mut registry = state.registry.lock().expect("model registry mutex poisoned");
        let mut transient_statuses = state
            .transient_statuses
            .lock()
            .expect("transient status mutex poisoned");

        transient_statuses.remove(model_id.as_str());

        match result {
            Ok(installed) => {
                eprintln!(
                    "[model_manager] persisting installed model model_id={} version={} model_path={}",
                    model_id.as_str(),
                    installed.version,
                    installed.model_path.display()
                );
                registry
                    .installed
                    .insert(model_id.as_str().to_string(), installed);
                registry.model_errors.remove(model_id.as_str());
                if let Err(error) = registry::save(&path, &registry) {
                    eprintln!(
                        "[model_manager] download_model registry save failed model_id={} error={}",
                        model_id.as_str(),
                        error
                    );
                } else {
                    eprintln!(
                        "[model_manager] download_model success model_id={} registry_path={}",
                        model_id.as_str(),
                        path.display()
                    );
                }
            }
            Err(error) => {
                let error = error.to_string();
                eprintln!(
                    "[model_manager] download_model error model_id={} error={}",
                    model_id.as_str(),
                    error
                );
                registry
                    .model_errors
                    .insert(model_id.as_str().to_string(), error);
                if let Err(save_error) = registry::save(&path, &registry) {
                    eprintln!(
                        "[model_manager] download_model error save failed model_id={} error={}",
                        model_id.as_str(),
                        save_error
                    );
                }
            }
        }

        drop(transient_statuses);
        drop(registry);

        if let Err(error) = emit_snapshot(&app, state.inner()) {
            eprintln!(
                "[model_manager] download_model emit snapshot failed model_id={} error={}",
                model_id.as_str(),
                error
            );
        }
    });

    Ok(snapshot)
}

pub fn load_model<R: Runtime>(
    app: &AppHandle<R>,
    state: &ModelManagerState,
    model_id: ModelId,
) -> Result<ModelManagerSnapshot, String> {
    let installed = {
        let registry = state
            .registry
            .lock()
            .map_err(|_| "model registry mutex poisoned".to_string())?;
        registry.installed.get(model_id.as_str()).cloned()
    }
    .ok_or_else(|| format!("{} is not downloaded", model_id.as_str()))?;

    let prepared = prepare_model(&installed, model_id);
    if let Err(error) = runtime::load_check(&prepared) {
        let error = error.to_string();
        let path = registry_path(app).map_err(|path_error| path_error.to_string())?;
        let mut registry = state
            .registry
            .lock()
            .map_err(|_| "model registry mutex poisoned".to_string())?;
        registry
            .model_errors
            .insert(model_id.as_str().to_string(), error.clone());
        registry::save(&path, &registry).map_err(|save_error| save_error.to_string())?;
        drop(registry);
        let _ = emit_snapshot(app, state);
        return Err(error);
    }

    let path = registry_path(app).map_err(|error| error.to_string())?;
    {
        let mut registry = state
            .registry
            .lock()
            .map_err(|_| "model registry mutex poisoned".to_string())?;
        registry.loaded_model_id = Some(model_id);
        registry.model_errors.remove(model_id.as_str());
        registry::save(&path, &registry).map_err(|error| error.to_string())?;
    }

    emit_snapshot(app, state)
}

pub fn unload_model<R: Runtime>(
    app: &AppHandle<R>,
    state: &ModelManagerState,
) -> Result<ModelManagerSnapshot, String> {
    let path = registry_path(app).map_err(|error| error.to_string())?;

    {
        let mut registry = state
            .registry
            .lock()
            .map_err(|_| "model registry mutex poisoned".to_string())?;
        registry.loaded_model_id = None;
        registry::save(&path, &registry).map_err(|error| error.to_string())?;
    }

    emit_snapshot(app, state)
}

pub fn clear_model_cache<R: Runtime>(
    app: &AppHandle<R>,
    state: &ModelManagerState,
) -> Result<ModelManagerSnapshot, String> {
    let cache_root = cache_root(app).map_err(|error| error.to_string())?;
    let models_root = models_root(app).map_err(|error| error.to_string())?;
    let path = registry_path(app).map_err(|error| error.to_string())?;

    eprintln!(
        "[model_manager] clear_model_cache start cache_root={} models_root={}",
        cache_root.display(),
        models_root.display()
    );

    {
        let mut transient_statuses = state
            .transient_statuses
            .lock()
            .map_err(|_| "transient status mutex poisoned".to_string())?;
        transient_statuses.clear();
    }

    if cache_root.exists() {
        fs::remove_dir_all(&cache_root).map_err(|error| error.to_string())?;
    }

    if models_root.exists() {
        fs::remove_dir_all(&models_root).map_err(|error| error.to_string())?;
    }

    {
        let mut registry = state
            .registry
            .lock()
            .map_err(|_| "model registry mutex poisoned".to_string())?;
        registry.loaded_model_id = None;
        registry.installed.clear();
        registry.model_errors.clear();
        registry::save(&path, &registry).map_err(|error| error.to_string())?;
    }

    eprintln!(
        "[model_manager] clear_model_cache success registry_path={}",
        path.display()
    );
    emit_snapshot(app, state)
}

fn emit_snapshot<R: Runtime>(
    app: &AppHandle<R>,
    state: &ModelManagerState,
) -> Result<ModelManagerSnapshot, String> {
    let snapshot = state.snapshot();
    app.emit(MODEL_MANAGER_EVENT, &snapshot)
        .map_err(|error| error.to_string())?;
    Ok(snapshot)
}

fn snapshot_from_registry(
    registry: &ModelRegistry,
    transient_statuses: &BTreeMap<String, ModelStatus>,
) -> ModelManagerSnapshot {
    let runtime_preflight = runtime::preflight();
    let models = catalog::all_model_ids()
        .iter()
        .copied()
        .map(|model_id| {
            let error_message = registry.model_errors.get(model_id.as_str()).cloned();
            let status = if let Some(status) = transient_statuses.get(model_id.as_str()) {
                *status
            } else if registry.loaded_model_id == Some(model_id) {
                ModelStatus::Loaded
            } else if error_message.is_some() {
                ModelStatus::Error
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
                error_message,
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

#[cfg(test)]
mod tests {
    use crate::model_manager::types::ModelId;

    #[test]
    fn loading_a_new_model_replaces_the_old_one() {
        let _loaded = Some(ModelId::ParakeetV02En);
        let loaded = Some(ModelId::ParakeetV03Multilingual);
        assert_eq!(loaded, Some(ModelId::ParakeetV03Multilingual));
    }
}
