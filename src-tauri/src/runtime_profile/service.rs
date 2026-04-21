use std::path::{Path, PathBuf};

use tauri::{AppHandle, Runtime};

use crate::{
    app_data_dir,
    model_manager::{
        types::{WorkerCudaStatus, WorkerRuntimeBootstrap, WorkerRuntimeBootstrapStatus},
    },
    runtime_profile::{
        catalog,
        detect,
        resolve,
        smoke_test,
        state::{load_state, runtime_profile_state_path_at_root, save_state},
        types::{
            RuntimeMachineFacts, RuntimeProfileCatalogEntry, RuntimeProfileCatalogOption,
            RuntimeProfileManifest, RuntimeProfileSnapshot, RuntimeProfileState,
            RuntimeSelectionMode, RuntimeSmokeTestResult,
        },
    },
    transcribe,
};

struct PreparedWorkerRuntime {
    python_exe: PathBuf,
    worker_cuda_status: WorkerCudaStatus,
}

struct RuntimeProfileAttempt {
    profile: RuntimeProfileCatalogEntry,
    reason: String,
}

pub fn load_runtime_profile_snapshot<R: Runtime>(
    app: &AppHandle<R>,
    bootstrap: &WorkerRuntimeBootstrap,
) -> Result<RuntimeProfileSnapshot, String> {
    let root = app_data_dir(app)?;
    let state = load_state(&runtime_profile_state_path_at_root(&root)).map_err(|error| error.to_string())?;
    let manifest = catalog::load_manifest().map_err(|error| error.to_string())?;

    Ok(RuntimeProfileSnapshot {
        selection_mode: state.selection_mode,
        selected_profile_id: state.selected_profile_id,
        resolved_auto_profile_id: state.resolved_auto_profile_id,
        active_profile_id: state.active_profile_id,
        last_successful_profile_id: state.last_successful_profile_id,
        manifest_version: state.manifest_version,
        actual_execution_device: state.actual_execution_device,
        last_test_result: state.last_test_result,
        last_error: state.last_error.or_else(|| bootstrap.message.clone()),
        profiles: manifest
            .profiles
            .into_iter()
            .map(|profile| RuntimeProfileCatalogOption {
                profile_id: profile.profile_id,
                label: profile.label,
                stability: profile.stability,
                auto_eligible: profile.auto_eligible,
            })
            .collect(),
    })
}

pub fn prepare_worker_runtime<R, F>(app: &AppHandle<R>, progress: F) -> Result<WorkerCudaStatus, String>
where
    R: Runtime,
    F: FnMut(WorkerRuntimeBootstrapStatus, Option<String>, Option<WorkerCudaStatus>),
{
    let root = app_data_dir(app)?;
    let prepared = prepare_worker_runtime_at_root(&root, progress)?;
    Ok(prepared.worker_cuda_status)
}

pub fn ensure_worker_python_ready(app_data_root: &Path) -> Result<PathBuf, String> {
    let prepared = prepare_worker_runtime_at_root(app_data_root, |_, _, _| {})?;
    Ok(prepared.python_exe)
}

pub fn set_selection_mode<R: Runtime>(
    app: &AppHandle<R>,
    mode: RuntimeSelectionMode,
) -> Result<(), String> {
    let root = app_data_dir(app)?;
    let manifest = catalog::load_manifest().map_err(|error| error.to_string())?;
    let path = runtime_profile_state_path_at_root(&root);
    let mut state = load_state(&path).map_err(|error| error.to_string())?;
    state.selection_mode = mode;
    state.last_error = None;
    if mode == RuntimeSelectionMode::Manual && state.selected_profile_id.is_none() {
        state.selected_profile_id = state
            .active_profile_id
            .clone()
            .or_else(|| manifest.profiles.first().map(|profile| profile.profile_id.clone()));
    }
    if mode == RuntimeSelectionMode::Auto {
        state.selected_profile_id = None;
    }
    save_state(&path, &state).map_err(|error| error.to_string())
}

pub fn select_profile<R: Runtime>(app: &AppHandle<R>, profile_id: &str) -> Result<(), String> {
    let root = app_data_dir(app)?;
    let manifest = catalog::load_manifest().map_err(|error| error.to_string())?;
    if !manifest.profiles.iter().any(|profile| profile.profile_id == profile_id) {
        return Err(format!("unknown runtime profile: {profile_id}"));
    }
    let path = runtime_profile_state_path_at_root(&root);
    let mut state = load_state(&path).map_err(|error| error.to_string())?;
    state.selection_mode = RuntimeSelectionMode::Manual;
    state.selected_profile_id = Some(profile_id.to_string());
    state.last_error = None;
    state.manifest_version = None;
    save_state(&path, &state).map_err(|error| error.to_string())
}

pub fn force_reprovision<R: Runtime>(app: &AppHandle<R>) -> Result<(), String> {
    let root = app_data_dir(app)?;
    let path = runtime_profile_state_path_at_root(&root);
    let mut state = load_state(&path).map_err(|error| error.to_string())?;
    state.manifest_version = None;
    state.last_error = None;
    save_state(&path, &state).map_err(|error| error.to_string())
}

pub fn reset_to_auto<R: Runtime>(app: &AppHandle<R>) -> Result<(), String> {
    let root = app_data_dir(app)?;
    let path = runtime_profile_state_path_at_root(&root);
    let mut state = load_state(&path).map_err(|error| error.to_string())?;
    state.selection_mode = RuntimeSelectionMode::Auto;
    state.selected_profile_id = None;
    state.manifest_version = None;
    state.last_error = None;
    save_state(&path, &state).map_err(|error| error.to_string())
}

pub fn record_worker_execution_device<R: Runtime>(app: &AppHandle<R>, device: &str) -> Result<(), String> {
    let root = app_data_dir(app)?;
    let path = runtime_profile_state_path_at_root(&root);
    let mut state = load_state(&path).map_err(|error| error.to_string())?;
    state.actual_execution_device = Some(normalize_worker_device(device));
    save_state(&path, &state).map_err(|error| error.to_string())
}

fn prepare_worker_runtime_at_root<F>(
    app_data_root: &Path,
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
    let python_exe = transcribe::verify_runtime_layout(app_data_root)?;
    let manifest = catalog::load_manifest().map_err(|error| error.to_string())?;
    let manifest_version = catalog::manifest_version().map_err(|error| error.to_string())?;
    let state_path = runtime_profile_state_path_at_root(app_data_root);
    let mut state = load_state(&state_path).map_err(|error| error.to_string())?;
    let facts = detect::detect_machine_facts();
    let attempts = resolve_attempts(&manifest, &mut state, &facts)?;
    let current_probe = transcribe::probe_worker_runtime_with_python(&python_exe)?;

    eprintln!(
        "[runtime-profile] detect gpu_name={} capability={} driver={} vendor={} error={}",
        facts.gpu_name.as_deref().unwrap_or("unknown"),
        facts.compute_capability
            .map(|value| value.to_string())
            .unwrap_or_else(|| "unknown".to_string()),
        facts.driver_version.as_deref().unwrap_or("unknown"),
        facts.gpu_vendor.as_deref().unwrap_or("unknown"),
        facts.detection_error.as_deref().unwrap_or("none")
    );

    if let Some(first) = attempts.first() {
        eprintln!(
            "[runtime-profile] resolve mode={} selected_profile={} reason={}",
            selection_mode_label(state.selection_mode),
            first.profile.profile_id,
            first.reason
        );
    }

    save_state(&state_path, &state).map_err(|error| error.to_string())?;

    if let Some(active_profile_id) = attempts.first().map(|attempt| attempt.profile.profile_id.as_str()) {
        if profile_is_current(&state, &manifest_version, active_profile_id, &current_probe) {
            eprintln!(
                "[runtime-profile] activate success profile_id={} device={}",
                active_profile_id,
                state
                    .actual_execution_device
                    .as_deref()
                    .unwrap_or(if current_probe.cuda_available { "gpu" } else { "cpu_fallback" })
            );
            return Ok(PreparedWorkerRuntime {
                python_exe,
                worker_cuda_status: transcribe::worker_cuda_status_from_probe(current_probe),
            });
        }
    }

    transcribe::ensure_pip_available(&python_exe, &mut progress)?;

    let smoke_test_script = transcribe::worker_smoke_test_path(app_data_root);
    let requirements_path = transcribe::worker_requirements_path(app_data_root);
    let is_auto = state.selection_mode == RuntimeSelectionMode::Auto;
    let mut last_error = None;

    for attempt in attempts {
        eprintln!(
            "[runtime-profile] install start profile_id={} reason={}",
            attempt.profile.profile_id, attempt.reason
        );
        progress(
            if attempt.profile.profile_id == "cpu" {
                WorkerRuntimeBootstrapStatus::FallingBackToCpu
            } else {
                WorkerRuntimeBootstrapStatus::InstallingCudaTorch
            },
            Some(format!("Installing runtime profile {}", attempt.profile.label)),
            None,
        );

        if let Err(error) = transcribe::install_runtime_profile_packages(&python_exe, &attempt.profile) {
            eprintln!(
                "[runtime-profile] install failed profile_id={} error={error}",
                attempt.profile.profile_id
            );
            last_error = Some(error.clone());
            update_failure_state(
                &state_path,
                &mut state,
                &attempt.profile.profile_id,
                Some(error.clone()),
                None,
                is_auto,
            )?;
            if is_auto {
                continue;
            }
            return Err(error);
        }

        progress(
            WorkerRuntimeBootstrapStatus::InstallingWorkerRequirements,
            Some("Installing Python worker dependencies".to_string()),
            None,
        );
        if let Err(error) = transcribe::install_worker_requirements(&python_exe, &requirements_path) {
            eprintln!(
                "[runtime-profile] install failed profile_id={} error={error}",
                attempt.profile.profile_id
            );
            last_error = Some(error.clone());
            update_failure_state(
                &state_path,
                &mut state,
                &attempt.profile.profile_id,
                Some(error.clone()),
                None,
                is_auto,
            )?;
            if is_auto {
                continue;
            }
            return Err(error);
        }

        progress(
            WorkerRuntimeBootstrapStatus::SmokeTesting,
            Some(format!("Smoke testing runtime profile {}", attempt.profile.label)),
            None,
        );
        eprintln!(
            "[runtime-profile] smoke-test start profile_id={}",
            attempt.profile.profile_id
        );
        let smoke_result = smoke_test::run_smoke_test(
            &python_exe,
            &smoke_test_script,
            &attempt.profile.profile_id,
            attempt.profile.profile_id != "cpu",
        )
        .map_err(|error| {
            format!(
                "smoke test process failed for profile {}: {error}",
                attempt.profile.profile_id
            )
        })?;

        if smoke_result.success {
            let worker_cuda_status = transcribe::worker_cuda_status_from_probe(
                transcribe::probe_worker_runtime_with_python(&python_exe)?,
            );
            state.active_profile_id = Some(attempt.profile.profile_id.clone());
            state.last_successful_profile_id = Some(attempt.profile.profile_id.clone());
            state.manifest_version = Some(manifest_version.clone());
            state.last_test_result = Some(smoke_result.clone());
            state.last_error = None;
            state.actual_execution_device = Some(smoke_result.device.clone());
            save_state(&state_path, &state).map_err(|error| error.to_string())?;
            eprintln!(
                "[runtime-profile] activate success profile_id={} device={}",
                attempt.profile.profile_id, smoke_result.device
            );
            return Ok(PreparedWorkerRuntime {
                python_exe,
                worker_cuda_status,
            });
        }

        let smoke_error = smoke_result
            .error
            .clone()
            .unwrap_or_else(|| "runtime smoke test failed".to_string());
        eprintln!(
            "[runtime-profile] smoke-test failed profile_id={} error={}",
            attempt.profile.profile_id, smoke_error
        );
        last_error = Some(smoke_error.clone());
        update_failure_state(
            &state_path,
            &mut state,
            &attempt.profile.profile_id,
            Some(smoke_error.clone()),
            Some(smoke_result),
            is_auto,
        )?;

        if !is_auto {
            return Err(smoke_error);
        }
    }

    Err(last_error.unwrap_or_else(|| "no runtime profile could be activated".to_string()))
}

fn update_failure_state(
    state_path: &Path,
    state: &mut RuntimeProfileState,
    profile_id: &str,
    error: Option<String>,
    smoke_result: Option<RuntimeSmokeTestResult>,
    is_auto: bool,
) -> Result<(), String> {
    state.active_profile_id = None;
    state.actual_execution_device = None;
    state.last_error = error;
    if smoke_result.is_some() {
        state.last_test_result = smoke_result;
    } else {
        state.last_test_result = Some(RuntimeSmokeTestResult {
            profile_id: profile_id.to_string(),
            success: false,
            device: if profile_id == "cpu" {
                "cpu_fallback".to_string()
            } else {
                "gpu".to_string()
            },
            torch_version: None,
            cuda_version: None,
            gpu_name: None,
            tested_at_epoch_ms: 0,
            error: state.last_error.clone(),
        });
    }
    if !is_auto {
        state.resolved_auto_profile_id = None;
    }
    save_state(state_path, state).map_err(|error| error.to_string())
}

fn resolve_attempts(
    manifest: &RuntimeProfileManifest,
    state: &mut RuntimeProfileState,
    facts: &RuntimeMachineFacts,
) -> Result<Vec<RuntimeProfileAttempt>, String> {
    match state.selection_mode {
        RuntimeSelectionMode::Manual => {
            let profile_id = state
                .selected_profile_id
                .as_deref()
                .ok_or_else(|| "manual runtime profile mode requires a selected profile".to_string())?;
            eprintln!("[runtime-profile] manual apply profile_id={profile_id}");
            let profile = manifest
                .profiles
                .iter()
                .find(|profile| profile.profile_id == profile_id)
                .cloned()
                .ok_or_else(|| format!("unknown runtime profile: {profile_id}"))?;
            Ok(vec![RuntimeProfileAttempt {
                profile,
                reason: "manual selection".to_string(),
            }])
        }
        RuntimeSelectionMode::Auto => {
            let (ranked, skipped) = resolve::rank_auto_candidates(manifest, facts);
            for skipped_profile in skipped {
                eprintln!(
                    "[runtime-profile] skip profile_id={} reason={}",
                    skipped_profile.profile_id, skipped_profile.reason
                );
            }
            state.resolved_auto_profile_id = ranked.first().map(|profile| profile.profile_id.clone());
            ranked
                .into_iter()
                .map(|resolved| {
                    manifest
                        .profiles
                        .iter()
                        .find(|profile| profile.profile_id == resolved.profile_id)
                        .cloned()
                        .map(|profile| RuntimeProfileAttempt {
                            profile,
                            reason: resolved.reason,
                        })
                        .ok_or_else(|| {
                            format!(
                                "resolved runtime profile missing from manifest: {}",
                                resolved.profile_id
                            )
                        })
                })
                .collect()
        }
    }
}

fn profile_is_current(
    state: &RuntimeProfileState,
    manifest_version: &str,
    profile_id: &str,
    probe: &transcribe::WorkerRuntimeProbeResult,
) -> bool {
    if state.manifest_version.as_deref() != Some(manifest_version) {
        return false;
    }
    if state.active_profile_id.as_deref() != Some(profile_id) {
        return false;
    }
    if !probe.dependencies_ready {
        return false;
    }
    if !state
        .last_test_result
        .as_ref()
        .is_some_and(|result| result.success && result.profile_id == profile_id)
    {
        return false;
    }
    if profile_id == "cpu" {
        return probe.cuda_version.is_none();
    }
    probe.cuda_version.is_some()
}

fn normalize_worker_device(device: &str) -> String {
    match device {
        "cuda" | "gpu" => "gpu".to_string(),
        "cpu" | "cpu_fallback" => "cpu_fallback".to_string(),
        other => other.to_string(),
    }
}

fn selection_mode_label(mode: RuntimeSelectionMode) -> &'static str {
    match mode {
        RuntimeSelectionMode::Auto => "auto",
        RuntimeSelectionMode::Manual => "manual",
    }
}

pub fn default_runtime_profile_snapshot() -> RuntimeProfileSnapshot {
    RuntimeProfileSnapshot {
        selection_mode: RuntimeSelectionMode::Auto,
        selected_profile_id: None,
        resolved_auto_profile_id: None,
        active_profile_id: None,
        last_successful_profile_id: None,
        manifest_version: None,
        actual_execution_device: None,
        last_test_result: None,
        last_error: None,
        profiles: Vec::new(),
    }
}
