pub mod audio;
pub mod child_process;
pub mod clipboard;
pub mod diagnostics;
pub mod history;
pub mod hotkey;
pub mod model_manager;
pub mod overlay;
pub mod python_runtime;
pub mod recordings;
pub mod runtime_profile;
pub mod settings;
pub mod state;
pub mod transcribe;

use std::{path::PathBuf, thread, time::Duration};

use tauri::{AppHandle, Manager, Runtime, State};

use crate::{
    audio::RecorderSession,
    model_manager::{
        service::{ModelManagerSnapshot, ModelManagerState},
        types::ModelId,
    },
    runtime_profile::types::RuntimeSelectionMode,
    settings::{AppSettings, AppSettingsState, SettingsRepository, DEFAULT_HOTKEY},
    state::{AppState, AppStateManager},
};

pub const MAIN_WINDOW_LABEL: &str = "main";
pub const OVERLAY_WINDOW_LABEL: &str = "overlay";
pub const APP_STATE_EVENT: &str = "app-state-changed";
pub const COMPLETED_RESET_DELAY_MS: u64 = 1_000;
pub const ERROR_RESET_DELAY_MS: u64 = 4_000;

pub struct RuntimeHandles {
    pub hotkey: std::sync::Mutex<Option<hotkey::HotkeyHandle>>,
    pub recorder: std::sync::Mutex<Option<RecorderSession>>,
    pub transcription: std::sync::Mutex<Option<transcribe::WorkerHandle>>,
}

impl Default for RuntimeHandles {
    fn default() -> Self {
        Self {
            hotkey: std::sync::Mutex::new(None),
            recorder: std::sync::Mutex::new(None),
            transcription: std::sync::Mutex::new(None),
        }
    }
}

pub fn configure_windows<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<()> {
    if let Some(window) = app.get_webview_window(MAIN_WINDOW_LABEL) {
        window.hide()?;
    }

    overlay::configure(app)?;
    overlay::sync_visibility_for_state(app, AppState::Ready)
}

pub fn transition_to<R: Runtime>(
    app: &AppHandle<R>,
    next: AppState,
    message: Option<String>,
) -> Result<bool, String> {
    let state = app.state::<AppStateManager>();
    let changed = state.transition(app, next, message)?;

    if changed {
        overlay::sync_visibility_for_state(app, next).map_err(|error| error.to_string())?;
    }

    Ok(changed)
}

pub fn recordings_dir<R: Runtime>(app: &AppHandle<R>) -> Result<PathBuf, String> {
    let base = app_data_dir(app)?;

    Ok(base.join("runtime").join("recordings"))
}

pub fn history_dir<R: Runtime>(app: &AppHandle<R>) -> Result<PathBuf, String> {
    Ok(app_data_dir(app)?.join("history"))
}

pub fn settings_path<R: Runtime>(app: &AppHandle<R>) -> Result<PathBuf, String> {
    Ok(app_data_dir(app)?.join("state").join("settings.json"))
}

pub fn startup_app_settings<R: Runtime>(
    app: &AppHandle<R>,
    state: &AppSettingsState,
) -> AppSettings {
    let loaded = settings_path(app).and_then(|path| SettingsRepository::new(path).load());
    match loaded {
        Ok(settings) => state.replace(settings.clone()).unwrap_or(settings),
        Err(error) => {
            diagnostics::log(format!("[settings] load failed; using defaults: {error}"));
            let defaults = AppSettings::default();
            state.replace(defaults.clone()).unwrap_or(defaults)
        }
    }
}

pub fn get_app_settings(state: &AppSettingsState) -> Result<AppSettings, String> {
    state.current()
}

pub fn set_save_history<R: Runtime>(
    app: &AppHandle<R>,
    state: &AppSettingsState,
    save_history: bool,
) -> Result<AppSettings, String> {
    state.set_save_history(&SettingsRepository::new(settings_path(app)?), save_history)
}

pub fn set_hotkey<R: Runtime>(
    app: &AppHandle<R>,
    state: &AppSettingsState,
    hotkey_value: String,
) -> Result<AppSettings, String> {
    if app.state::<AppStateManager>().current() != AppState::Ready {
        return Err("Wait for the current recording or transcription to finish".to_string());
    }

    let handles = app.state::<RuntimeHandles>();
    let mut handle = handles
        .hotkey
        .lock()
        .map_err(|_| "hotkey mutex poisoned".to_string())?;
    let handle = handle
        .as_mut()
        .ok_or_else(|| "The global shortcut is currently unavailable".to_string())?;
    let repository = SettingsRepository::new(settings_path(app)?);
    let mut confirmed = None;

    let cleanup_warning = handle.replace_with(&hotkey_value, |normalized| {
        let settings = state.set_hotkey(&repository, normalized.to_string())?;
        confirmed = Some(settings);
        Ok(())
    })?;
    if let Some(error) = cleanup_warning {
        diagnostics::log(format!(
            "[hotkey] previous registration cleanup failed: {error}"
        ));
    }

    confirmed.ok_or_else(|| "Shortcut settings were not confirmed".to_string())
}

pub fn register_startup_hotkey<R: Runtime>(
    app: &AppHandle<R>,
    state: &AppSettingsState,
    settings: &AppSettings,
) -> Result<hotkey::HotkeyHandle, String> {
    match hotkey::register(app.clone(), &settings.hotkey) {
        Ok(handle) => Ok(handle),
        Err(error) if settings.hotkey == DEFAULT_HOTKEY => Err(error),
        Err(error) => {
            diagnostics::log(format!(
                "[hotkey] saved shortcut unavailable; trying default: {error}"
            ));
            let fallback = hotkey::register(app.clone(), DEFAULT_HOTKEY)?;
            state.set_hotkey(
                &SettingsRepository::new(settings_path(app)?),
                DEFAULT_HOTKEY.to_string(),
            )?;
            Ok(fallback)
        }
    }
}

pub fn list_history<R: Runtime>(
    app: &AppHandle<R>,
    offset: usize,
) -> Result<history::HistoryPage, String> {
    history::HistoryRepository::new(history_dir(app)?).list(offset, history::MAX_PAGE_SIZE)
}

pub fn read_history_entry<R: Runtime>(
    app: &AppHandle<R>,
    id: &str,
) -> Result<history::HistoryTranscript, String> {
    history::HistoryRepository::new(history_dir(app)?).read(id)
}

pub fn delete_history_entry<R: Runtime>(app: &AppHandle<R>, id: &str) -> Result<(), String> {
    history::HistoryRepository::new(history_dir(app)?).delete(id)
}

pub fn clear_history<R: Runtime>(app: &AppHandle<R>) -> Result<(), String> {
    history::HistoryRepository::new(history_dir(app)?).clear()
}

pub fn repair_history_on_startup<R: Runtime>(app: &AppHandle<R>) -> Result<(), String> {
    history::HistoryRepository::new(history_dir(app)?).startup_repair()
}

pub fn app_data_dir<R: Runtime>(app: &AppHandle<R>) -> Result<PathBuf, String> {
    app.path()
        .app_local_data_dir()
        .map_err(|error| error.to_string())
}

pub fn bootstrap_transcription_runtime<R: Runtime>(app: &AppHandle<R>) -> Result<(), String> {
    let root = app_data_dir(app)?;
    transcribe::bootstrap_runtime(&root)?;
    let _python_exe = python_runtime::ensure_python_runtime(app, &root)?;
    Ok(())
}

pub fn start_recording<R: Runtime>(app: &AppHandle<R>) -> Result<(), String> {
    let handles: State<'_, RuntimeHandles> = app.state();
    if handles
        .transcription
        .lock()
        .map_err(|_| "transcription mutex poisoned".to_string())?
        .is_none()
    {
        let error = "Load a model before recording".to_string();
        transition_to(app, AppState::Error, Some(error.clone()))?;
        schedule_error_reset(app.clone());
        return Err(error);
    }

    if !transition_to(app, AppState::Listening, None)? {
        return Ok(());
    }

    let output_dir = recordings_dir(app)?;
    match RecorderSession::start(&output_dir) {
        Ok(session) => {
            *handles
                .recorder
                .lock()
                .map_err(|_| "recorder mutex poisoned".to_string())? = Some(session);
            Ok(())
        }
        Err(error) => {
            transition_to(app, AppState::Error, Some(error.clone()))?;
            schedule_error_reset(app.clone());
            Err(error)
        }
    }
}

pub fn stop_recording<R: Runtime>(app: &AppHandle<R>) -> Result<(), String> {
    let handles: State<'_, RuntimeHandles> = app.state();
    let session = handles
        .recorder
        .lock()
        .map_err(|_| "recorder mutex poisoned".to_string())?
        .take()
        .ok_or_else(|| "No active recorder session".to_string())?;

    let output_dir = recordings_dir(app)?;
    match session.stop(&output_dir) {
        Ok(result) => {
            let _temporary_recording = recordings::TemporaryRecording::new(result.path.clone());
            eprintln!(
                "[recording] captured sample_rate={} sample_count={}",
                result.sample_rate, result.sample_count
            );

            transition_to(app, AppState::Processing, None)?;

            match transcribe::transcribe_file(app, &result.path) {
                Ok(transcription) => {
                    let save_history = app
                        .state::<AppSettingsState>()
                        .current()
                        .map(|settings| settings.save_history)
                        .unwrap_or_else(|error| {
                            diagnostics::log(format!(
                                "[settings] read failed; skipping history save: {error}"
                            ));
                            false
                        });
                    if let Err(error) = history_dir(app).and_then(|directory| {
                        history::HistoryRepository::new(directory)
                            .save_if_enabled(
                                save_history,
                                &transcription.text,
                                result.duration_ms,
                                None,
                            )
                            .map(|_| ())
                    }) {
                        diagnostics::log(format!("[history] save failed: {error}"));
                    }
                    clipboard::copy_text(&transcription.text)?;

                    let completed_message = if clipboard::paste_clipboard().is_ok() {
                        "Pasted".to_string()
                    } else {
                        "Copied to clipboard".to_string()
                    };

                    transition_to(app, AppState::Completed, Some(completed_message))?;
                    schedule_completed_reset(app.clone());
                    Ok(())
                }
                Err(error) => {
                    transition_to(app, AppState::Error, Some(error.clone()))?;
                    schedule_error_reset(app.clone());
                    Err(error)
                }
            }
        }
        Err(error) => {
            transition_to(app, AppState::Error, Some(error.clone()))?;
            schedule_error_reset(app.clone());
            Err(error)
        }
    }
}

pub fn cleanup_recordings_on_startup<R: Runtime>(app: &AppHandle<R>) -> Result<(), String> {
    let removed = recordings::cleanup_abandoned_recordings(&recordings_dir(app)?)?;
    if removed > 0 {
        diagnostics::log(format!("[recordings] removed_abandoned count={removed}"));
    }
    Ok(())
}

fn schedule_reset<R: Runtime>(app: AppHandle<R>, delay_ms: u64) {
    thread::spawn(move || {
        thread::sleep(Duration::from_millis(delay_ms));
        let _ = transition_to(&app, AppState::Ready, None);
    });
}

fn schedule_completed_reset<R: Runtime>(app: AppHandle<R>) {
    schedule_reset(app, COMPLETED_RESET_DELAY_MS);
}

fn schedule_error_reset<R: Runtime>(app: AppHandle<R>) {
    schedule_reset(app, ERROR_RESET_DELAY_MS);
}

pub fn reveal_settings<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<()> {
    if let Some(window) = app.get_webview_window(MAIN_WINDOW_LABEL) {
        if window.is_minimized()? {
            window.unminimize()?;
        }

        window.show()?;
        window.set_focus()?;
    }

    Ok(())
}

pub fn show_settings(app: AppHandle) -> Result<(), String> {
    reveal_settings(&app).map_err(|error| error.to_string())
}

pub fn show_overlay(app: AppHandle) -> Result<(), String> {
    overlay::show_overlay(&app).map_err(|error| error.to_string())
}

pub fn hide_overlay(app: AppHandle) -> Result<(), String> {
    overlay::hide_overlay(&app).map_err(|error| error.to_string())
}

pub fn toggle_overlay(app: AppHandle) -> Result<bool, String> {
    overlay::toggle_overlay(&app).map_err(|error| error.to_string())
}

pub fn startup_model_manager<R: Runtime>(
    app: &AppHandle<R>,
) -> Result<ModelManagerSnapshot, String> {
    let state: State<'_, ModelManagerState> = app.state();
    model_manager::service::startup_reconcile(app, state.inner())
}

pub fn list_models<R: Runtime>(
    app: &AppHandle<R>,
    state: &ModelManagerState,
) -> ModelManagerSnapshot {
    model_manager::service::list_models(app, state)
}

pub fn open_diagnostics_log<R: Runtime>(app: &AppHandle<R>) -> Result<(), String> {
    diagnostics::open_log_in_file_manager(app)
}

pub fn preview_issue_report<R: Runtime>(
    app: &AppHandle<R>,
    state: &ModelManagerState,
) -> diagnostics::IssueReport {
    let snapshot = list_models(app, state);
    diagnostics::build_issue_report(diagnostics::IssueReportInput {
        app_version: env!("CARGO_PKG_VERSION").to_string(),
        operating_system: snapshot.operating_system,
        selected_model_id: snapshot.selected_model_id.as_str().to_string(),
        loaded_model_id: snapshot
            .loaded_model_id
            .map(|model_id| model_id.as_str().to_string()),
        execution_device: snapshot.live_execution_device,
    })
}

pub fn select_model<R: Runtime>(
    app: &AppHandle<R>,
    state: &ModelManagerState,
    model_id: ModelId,
) -> Result<ModelManagerSnapshot, String> {
    model_manager::service::select_model(app, state, model_id)
}

pub fn download_model<R: Runtime>(
    app: &AppHandle<R>,
    state: &ModelManagerState,
    model_id: ModelId,
) -> Result<ModelManagerSnapshot, String> {
    model_manager::service::download_model(app, state, model_id)
}

pub fn load_model<R: Runtime>(
    app: &AppHandle<R>,
    state: &ModelManagerState,
    model_id: ModelId,
) -> Result<ModelManagerSnapshot, String> {
    model_manager::service::load_model(app, state, model_id)
}

pub fn unload_model<R: Runtime>(
    app: &AppHandle<R>,
    state: &ModelManagerState,
) -> Result<ModelManagerSnapshot, String> {
    model_manager::service::unload_model(app, state)
}

pub fn get_model_status<R: Runtime>(
    app: &AppHandle<R>,
    state: &ModelManagerState,
) -> ModelManagerSnapshot {
    model_manager::service::get_model_status(app, state)
}

pub fn clear_model_cache<R: Runtime>(
    app: &AppHandle<R>,
    state: &ModelManagerState,
) -> Result<ModelManagerSnapshot, String> {
    model_manager::service::clear_model_cache(app, state)
}

pub fn set_runtime_selection_mode<R: Runtime>(
    app: &AppHandle<R>,
    state: &ModelManagerState,
    selection_mode: RuntimeSelectionMode,
) -> Result<ModelManagerSnapshot, String> {
    model_manager::service::set_runtime_selection_mode(app, state, selection_mode)
}

pub fn select_runtime_profile<R: Runtime>(
    app: &AppHandle<R>,
    state: &ModelManagerState,
    profile_id: String,
) -> Result<ModelManagerSnapshot, String> {
    model_manager::service::select_runtime_profile(app, state, profile_id)
}

pub fn apply_runtime_profile_now<R: Runtime>(
    app: &AppHandle<R>,
    state: &ModelManagerState,
) -> Result<ModelManagerSnapshot, String> {
    model_manager::service::apply_runtime_profile_now(app, state)
}

pub fn test_runtime_profile<R: Runtime>(
    app: &AppHandle<R>,
    state: &ModelManagerState,
) -> Result<ModelManagerSnapshot, String> {
    model_manager::service::test_runtime_profile(app, state)
}

pub fn redetect_runtime_profile<R: Runtime>(
    app: &AppHandle<R>,
    state: &ModelManagerState,
) -> Result<ModelManagerSnapshot, String> {
    model_manager::service::redetect_runtime_profile(app, state)
}

pub fn reset_runtime_profile_to_auto<R: Runtime>(
    app: &AppHandle<R>,
    state: &ModelManagerState,
) -> Result<ModelManagerSnapshot, String> {
    model_manager::service::reset_runtime_profile_to_auto(app, state)
}
