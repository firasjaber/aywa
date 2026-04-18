pub mod audio;
pub mod clipboard;
pub mod hotkey;
pub mod model_manager;
pub mod overlay;
pub mod state;
pub mod transcribe;

use std::{path::PathBuf, thread, time::Duration};

use tauri::{AppHandle, Manager, Runtime, State};

use crate::{
    audio::RecorderSession,
    state::{AppState, AppStateManager},
};

pub const MAIN_WINDOW_LABEL: &str = "main";
pub const OVERLAY_WINDOW_LABEL: &str = "overlay";
pub const APP_STATE_EVENT: &str = "app-state-changed";
pub const ERROR_RESET_DELAY_MS: u64 = 1_500;

pub struct RuntimeHandles {
    pub hotkey: std::sync::Mutex<Option<hotkey::HotkeyHandle>>,
    pub recorder: std::sync::Mutex<Option<RecorderSession>>,
}

impl Default for RuntimeHandles {
    fn default() -> Self {
        Self {
            hotkey: std::sync::Mutex::new(None),
            recorder: std::sync::Mutex::new(None),
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
    let base = app
        .path()
        .app_local_data_dir()
        .map_err(|error| error.to_string())?;

    Ok(base.join("runtime").join("recordings"))
}

pub fn start_recording<R: Runtime>(app: &AppHandle<R>) -> Result<(), String> {
    if !transition_to(app, AppState::Listening, None)? {
        return Ok(());
    }

    let output_dir = recordings_dir(app)?;
    match RecorderSession::start(&output_dir) {
        Ok(session) => {
            let handles: State<'_, RuntimeHandles> = app.state();
            *handles
                .recorder
                .lock()
                .map_err(|_| "recorder mutex poisoned".to_string())? = Some(session);
            Ok(())
        }
        Err(error) => {
            transition_to(app, AppState::Error, Some(error.clone()))?;
            schedule_ready_reset(app.clone());
            Err(error)
        }
    }
}

pub fn stop_recording<R: Runtime>(app: &AppHandle<R>) -> Result<(), String> {
    if !transition_to(app, AppState::Saving, None)? {
        return Ok(());
    }

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
            println!(
                "saved recording to {} (sample_rate={}, sample_count={})",
                result.path.display(),
                result.sample_rate,
                result.sample_count
            );
            let _ = transition_to(app, AppState::Ready, None)?;
            Ok(())
        }
        Err(error) => {
            transition_to(app, AppState::Error, Some(error.clone()))?;
            schedule_ready_reset(app.clone());
            Err(error)
        }
    }
}

fn schedule_ready_reset<R: Runtime>(app: AppHandle<R>) {
    thread::spawn(move || {
        thread::sleep(Duration::from_millis(ERROR_RESET_DELAY_MS));
        let _ = transition_to(&app, AppState::Ready, None);
    });
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
