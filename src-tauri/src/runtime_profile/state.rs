use std::{
    fs,
    path::{Path, PathBuf},
};

use tauri::{AppHandle, Manager, Runtime};

use crate::runtime_profile::types::RuntimeProfileState;

pub const RUNTIME_PROFILE_STATE_FILE: &str = "runtime_profile_state.json";

pub fn runtime_profile_state_path_at_root(app_data_root: &Path) -> PathBuf {
    app_data_root.join("state").join(RUNTIME_PROFILE_STATE_FILE)
}

pub fn runtime_profile_state_path<R: Runtime>(app: &AppHandle<R>) -> Result<PathBuf, String> {
    Ok(runtime_profile_state_path_at_root(
        &app.path()
            .app_local_data_dir()
            .map_err(|error| error.to_string())?,
    ))
}

pub fn load_state(path: &Path) -> anyhow::Result<RuntimeProfileState> {
    if !path.exists() {
        return Ok(RuntimeProfileState::default());
    }

    Ok(serde_json::from_slice(&fs::read(path)?)?)
}

pub fn save_state(path: &Path, state: &RuntimeProfileState) -> anyhow::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    fs::write(path, serde_json::to_vec_pretty(state)?)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{load_state, save_state};
    use crate::runtime_profile::types::{
        RuntimeProfileState, RuntimeSelectionMode, RuntimeSmokeTestResult,
    };

    #[test]
    fn runtime_profile_state_round_trips_on_disk() {
        let temp = std::env::temp_dir().join(format!(
            "aywa-runtime-profile-state-test-{}-{}.json",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let expected = RuntimeProfileState {
            selection_mode: RuntimeSelectionMode::Manual,
            selected_profile_id: Some("nvidia-latest".to_string()),
            resolved_auto_profile_id: Some("nvidia-current".to_string()),
            active_profile_id: Some("nvidia-latest".to_string()),
            last_successful_profile_id: Some("nvidia-current".to_string()),
            manifest_version: Some("2026-04-21".to_string()),
            actual_execution_device: Some("gpu".to_string()),
            last_test_result: Some(RuntimeSmokeTestResult {
                profile_id: "nvidia-latest".to_string(),
                success: false,
                device: "gpu".to_string(),
                torch_version: Some("2.7.0".to_string()),
                cuda_version: Some("12.8".to_string()),
                gpu_name: Some("NVIDIA GeForce RTX 5090".to_string()),
                tested_at_epoch_ms: 1_776_729_600_000,
                error: Some("CUDA kernel launch failed".to_string()),
            }),
            last_error: Some("smoke test failed".to_string()),
        };

        save_state(&temp, &expected).unwrap();
        let actual = load_state(&temp).unwrap();
        std::fs::remove_file(&temp).unwrap();

        assert_eq!(actual, expected);
    }
}
