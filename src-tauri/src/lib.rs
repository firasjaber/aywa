pub mod audio;
pub mod clipboard;
pub mod hotkey;
pub mod model_manager;
pub mod overlay;
pub mod state;
pub mod transcribe;

use tauri::{AppHandle, Manager, Runtime};

use crate::state::{AppState, AppStateManager};

pub const MAIN_WINDOW_LABEL: &str = "main";
pub const OVERLAY_WINDOW_LABEL: &str = "overlay";
pub const APP_STATE_EVENT: &str = "app-state-changed";
pub const DUMMY_TRANSCRIPT: &str = "This is dummy text from OK phase one.";

pub struct RuntimeHandles {
    pub hotkey: std::sync::Mutex<Option<hotkey::HotkeyHandle>>,
}

impl Default for RuntimeHandles {
    fn default() -> Self {
        Self {
            hotkey: std::sync::Mutex::new(None),
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

pub fn transition_to<R: Runtime>(app: &AppHandle<R>, next: AppState) -> Result<bool, String> {
    let state = app.state::<AppStateManager>();
    let changed = state.transition(app, next)?;

    if changed {
        overlay::sync_visibility_for_state(app, next).map_err(|error| error.to_string())?;
    }

    Ok(changed)
}

pub fn run_fake_transcription_flow<R: Runtime>(app: AppHandle<R>) {
    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_secs(2));

        let _ = crate::clipboard::copy_text(DUMMY_TRANSCRIPT);
        let _ = transition_to(&app, state::AppState::Completed);

        std::thread::sleep(std::time::Duration::from_secs(2));
        let _ = transition_to(&app, state::AppState::Ready);
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
