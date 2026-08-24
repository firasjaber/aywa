use std::{
    fs,
    os::windows::ffi::OsStrExt,
    path::{Path, PathBuf},
    sync::Mutex,
};

use windows_sys::Win32::Storage::FileSystem::{
    MoveFileExW, MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH,
};

pub const SETTINGS_SCHEMA_VERSION: u32 = 1;
pub const DEFAULT_HOTKEY: &str = "control+alt+Space";

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
#[serde(default, rename_all = "camelCase")]
pub struct AppSettings {
    pub version: u32,
    pub save_history: bool,
    pub hotkey: String,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            version: SETTINGS_SCHEMA_VERSION,
            save_history: true,
            hotkey: DEFAULT_HOTKEY.to_string(),
        }
    }
}

pub struct SettingsRepository {
    path: PathBuf,
}

pub struct AppSettingsState {
    current: Mutex<AppSettings>,
}

impl SettingsRepository {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }

    pub fn load(&self) -> Result<AppSettings, String> {
        let contents = match fs::read(&self.path) {
            Ok(contents) => contents,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Ok(AppSettings::default());
            }
            Err(error) => return Err(format!("failed to read app settings: {error}")),
        };
        let settings: AppSettings = serde_json::from_slice(&contents)
            .map_err(|error| format!("failed to read app settings: {error}"))?;
        if settings.version != SETTINGS_SCHEMA_VERSION {
            return Err(format!(
                "unsupported settings version: {}",
                settings.version
            ));
        }
        Ok(settings)
    }

    pub fn save(&self, settings: &AppSettings) -> Result<(), String> {
        let parent = self
            .path
            .parent()
            .ok_or_else(|| "settings path has no parent directory".to_string())?;
        fs::create_dir_all(parent)
            .map_err(|error| format!("failed to create settings directory: {error}"))?;
        let contents = serde_json::to_vec_pretty(settings)
            .map_err(|error| format!("failed to encode app settings: {error}"))?;
        atomic_write(&self.path, &contents)
    }
}

impl Default for AppSettingsState {
    fn default() -> Self {
        Self {
            current: Mutex::new(AppSettings::default()),
        }
    }
}

impl AppSettingsState {
    pub fn current(&self) -> Result<AppSettings, String> {
        self.current
            .lock()
            .map_err(|_| "app settings mutex poisoned".to_string())
            .map(|settings| settings.clone())
    }

    pub fn replace(&self, settings: AppSettings) -> Result<AppSettings, String> {
        let mut current = self
            .current
            .lock()
            .map_err(|_| "app settings mutex poisoned".to_string())?;
        *current = settings.clone();
        Ok(settings)
    }

    pub fn set_save_history(
        &self,
        repository: &SettingsRepository,
        save_history: bool,
    ) -> Result<AppSettings, String> {
        let mut current = self
            .current
            .lock()
            .map_err(|_| "app settings mutex poisoned".to_string())?;
        let mut next = current.clone();
        next.save_history = save_history;
        repository.save(&next)?;
        *current = next.clone();
        Ok(next)
    }

    pub fn set_hotkey(
        &self,
        repository: &SettingsRepository,
        hotkey: String,
    ) -> Result<AppSettings, String> {
        let mut current = self
            .current
            .lock()
            .map_err(|_| "app settings mutex poisoned".to_string())?;
        let mut next = current.clone();
        next.hotkey = hotkey;
        repository.save(&next)?;
        *current = next.clone();
        Ok(next)
    }
}

fn atomic_write(path: &Path, contents: &[u8]) -> Result<(), String> {
    let temporary = path.with_extension("json.tmp");
    fs::write(&temporary, contents)
        .map_err(|error| format!("failed to write app settings: {error}"))?;
    let source = temporary
        .as_os_str()
        .encode_wide()
        .chain(Some(0))
        .collect::<Vec<_>>();
    let destination = path
        .as_os_str()
        .encode_wide()
        .chain(Some(0))
        .collect::<Vec<_>>();
    let replaced = unsafe {
        MoveFileExW(
            source.as_ptr(),
            destination.as_ptr(),
            MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
        )
    };
    if replaced == 0 {
        let error = std::io::Error::last_os_error();
        let _ = fs::remove_file(&temporary);
        return Err(format!("failed to finalize app settings: {error}"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{
        AppSettings, AppSettingsState, SettingsRepository, DEFAULT_HOTKEY, SETTINGS_SCHEMA_VERSION,
    };

    #[test]
    fn defaults_enable_history_saving() {
        let settings = AppSettings::default();
        assert_eq!(settings.version, SETTINGS_SCHEMA_VERSION);
        assert!(settings.save_history);
        assert_eq!(settings.hotkey, DEFAULT_HOTKEY);
    }

    #[test]
    fn missing_file_loads_defaults_without_creating_a_file() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("state/settings.json");
        let repository = SettingsRepository::new(path.clone());
        assert_eq!(repository.load().unwrap(), AppSettings::default());
        assert!(!path.exists());
    }

    #[test]
    fn settings_round_trip_and_replace_the_previous_value() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("state/settings.json");
        let repository = SettingsRepository::new(path.clone());
        let disabled = AppSettings {
            save_history: false,
            ..AppSettings::default()
        };
        repository.save(&disabled).unwrap();
        assert_eq!(repository.load().unwrap(), disabled);
        let contents = std::fs::read_to_string(&path).unwrap();
        assert!(contents.contains(r#""saveHistory": false"#));
        assert!(!contents.contains("save_history"));
        repository.save(&AppSettings::default()).unwrap();
        assert_eq!(repository.load().unwrap(), AppSettings::default());
        assert!(!path.with_extension("json.tmp").exists());
    }

    #[test]
    fn settings_without_hotkey_keep_history_and_use_default_shortcut() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("state/settings.json");
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, r#"{"version":1,"saveHistory":false}"#).unwrap();

        let loaded = SettingsRepository::new(path).load().unwrap();
        assert!(!loaded.save_history);
        assert_eq!(loaded.hotkey, DEFAULT_HOTKEY);
    }

    #[test]
    fn custom_hotkey_round_trips_in_the_shared_settings_file() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("state/settings.json");
        let repository = SettingsRepository::new(path.clone());
        let settings = AppSettings {
            hotkey: "control+shift+KeyD".to_string(),
            ..AppSettings::default()
        };

        repository.save(&settings).unwrap();
        assert_eq!(repository.load().unwrap(), settings);
        let contents = std::fs::read_to_string(path).unwrap();
        assert!(contents.contains(r#""saveHistory": true"#));
        assert!(contents.contains(r#""hotkey": "control+shift+KeyD""#));
    }

    #[test]
    fn invalid_json_and_unknown_versions_are_rejected() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("state/settings.json");
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, "not json").unwrap();
        let repository = SettingsRepository::new(path.clone());
        assert!(repository.load().unwrap_err().contains("read app settings"));
        std::fs::write(&path, r#"{"version":2,"saveHistory":false}"#).unwrap();
        assert!(repository
            .load()
            .unwrap_err()
            .contains("unsupported settings version"));
    }

    #[test]
    fn failed_save_does_not_change_confirmed_in_memory_state() {
        let directory = tempfile::tempdir().unwrap();
        let unwritable_path = directory.path().join("state");
        std::fs::write(&unwritable_path, "blocks directory creation").unwrap();
        let repository = SettingsRepository::new(unwritable_path.join("settings.json"));
        let state = AppSettingsState::default();
        assert!(state.set_save_history(&repository, false).is_err());
        assert!(state.current().unwrap().save_history);
    }

    #[test]
    fn failed_hotkey_save_keeps_confirmed_in_memory_settings() {
        let directory = tempfile::tempdir().unwrap();
        let blocked = directory.path().join("state");
        std::fs::write(&blocked, "blocks directory creation").unwrap();
        let state = AppSettingsState::default();

        assert!(state
            .set_hotkey(
                &SettingsRepository::new(blocked.join("settings.json")),
                "control+shift+KeyD".to_string(),
            )
            .is_err());
        assert_eq!(state.current().unwrap().hotkey, DEFAULT_HOTKEY);
    }
}
