// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

#[cfg(not(target_os = "windows"))]
compile_error!("Aywa supports Windows only.");

use tauri::{
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Manager, State,
};

use aywa_lib::{
    model_manager::{
        service::{ModelManagerSnapshot, ModelManagerState},
        types::ModelId,
    },
    runtime_profile::types::RuntimeSelectionMode,
    settings::{AppSettings, AppSettingsState, TranscriptionRule},
    state::AppStateManager,
    RuntimeHandles,
};

const SETTINGS_MENU_ID: &str = "settings";
const QUIT_MENU_ID: &str = "quit";

fn build_tray<R: tauri::Runtime>(app: &AppHandle<R>) -> tauri::Result<()> {
    let settings = MenuItem::with_id(app, SETTINGS_MENU_ID, "Settings", true, None::<&str>)?;
    let separator = PredefinedMenuItem::separator(app)?;
    let quit = MenuItem::with_id(app, QUIT_MENU_ID, "Quit", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&settings, &separator, &quit])?;
    let icon = tauri::image::Image::from_bytes(include_bytes!(
        "../../static/logo/tray/aywa-tray-color-32.png"
    ))?;

    TrayIconBuilder::new()
        .icon(icon)
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id.as_ref() {
            SETTINGS_MENU_ID => {
                let _ = aywa_lib::reveal_settings(app);
            }
            QUIT_MENU_ID => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                let _ = aywa_lib::reveal_settings(tray.app_handle());
            }
        })
        .build(app)?;

    Ok(())
}

#[tauri::command]
fn show_settings(app: AppHandle) -> Result<(), String> {
    aywa_lib::show_settings(app)
}

#[tauri::command]
fn show_overlay(app: AppHandle) -> Result<(), String> {
    aywa_lib::show_overlay(app)
}

#[tauri::command]
fn hide_overlay(app: AppHandle) -> Result<(), String> {
    aywa_lib::hide_overlay(app)
}

#[tauri::command]
fn toggle_overlay(app: AppHandle) -> Result<bool, String> {
    aywa_lib::toggle_overlay(app)
}

#[tauri::command]
fn get_current_state(state: State<'_, AppStateManager>) -> aywa_lib::state::AppStatePayload {
    aywa_lib::state::get_current_state(state)
}

#[tauri::command]
fn list_history(app: AppHandle, offset: usize) -> Result<aywa_lib::history::HistoryPage, String> {
    aywa_lib::list_history(&app, offset)
}

#[tauri::command]
fn read_history_entry(
    app: AppHandle,
    id: String,
) -> Result<aywa_lib::history::HistoryTranscript, String> {
    aywa_lib::read_history_entry(&app, &id)
}

#[tauri::command]
fn delete_history_entry(app: AppHandle, id: String) -> Result<(), String> {
    aywa_lib::delete_history_entry(&app, &id)
}

#[tauri::command]
fn clear_history(app: AppHandle) -> Result<(), String> {
    aywa_lib::clear_history(&app)
}

#[tauri::command]
fn get_app_settings(state: State<'_, AppSettingsState>) -> Result<AppSettings, String> {
    aywa_lib::get_app_settings(state.inner())
}

#[tauri::command]
fn set_save_history(
    app: AppHandle,
    state: State<'_, AppSettingsState>,
    save_history: bool,
) -> Result<AppSettings, String> {
    aywa_lib::set_save_history(&app, state.inner(), save_history)
}

#[tauri::command]
fn set_rules(
    app: AppHandle,
    state: State<'_, AppSettingsState>,
    rules: Vec<TranscriptionRule>,
) -> Result<AppSettings, String> {
    aywa_lib::set_rules(&app, state.inner(), rules)
}

#[tauri::command]
fn set_hotkey(
    app: AppHandle,
    state: State<'_, AppSettingsState>,
    hotkey: String,
) -> Result<AppSettings, String> {
    aywa_lib::set_hotkey(&app, state.inner(), hotkey)
}

#[tauri::command]
fn list_models(app: AppHandle, state: State<'_, ModelManagerState>) -> ModelManagerSnapshot {
    aywa_lib::list_models(&app, state.inner())
}

#[tauri::command]
fn open_diagnostics_log(app: AppHandle) -> Result<(), String> {
    aywa_lib::open_diagnostics_log(&app)
}

#[tauri::command]
fn preview_issue_report(
    app: AppHandle,
    state: State<'_, ModelManagerState>,
) -> aywa_lib::diagnostics::IssueReport {
    aywa_lib::preview_issue_report(&app, state.inner())
}

#[tauri::command]
fn select_model(
    app: AppHandle,
    state: State<'_, ModelManagerState>,
    model_id: ModelId,
) -> Result<ModelManagerSnapshot, String> {
    aywa_lib::select_model(&app, state.inner(), model_id)
}

#[tauri::command]
fn download_model(
    app: AppHandle,
    state: State<'_, ModelManagerState>,
    model_id: ModelId,
) -> Result<ModelManagerSnapshot, String> {
    aywa_lib::download_model(&app, state.inner(), model_id)
}

#[tauri::command]
fn load_model(
    app: AppHandle,
    state: State<'_, ModelManagerState>,
    model_id: ModelId,
) -> Result<ModelManagerSnapshot, String> {
    aywa_lib::load_model(&app, state.inner(), model_id)
}

#[tauri::command]
fn unload_model(
    app: AppHandle,
    state: State<'_, ModelManagerState>,
) -> Result<ModelManagerSnapshot, String> {
    aywa_lib::unload_model(&app, state.inner())
}

#[tauri::command]
fn get_model_status(app: AppHandle, state: State<'_, ModelManagerState>) -> ModelManagerSnapshot {
    aywa_lib::get_model_status(&app, state.inner())
}

#[tauri::command]
fn clear_model_cache(
    app: AppHandle,
    state: State<'_, ModelManagerState>,
) -> Result<ModelManagerSnapshot, String> {
    aywa_lib::clear_model_cache(&app, state.inner())
}

#[tauri::command]
fn set_runtime_selection_mode(
    app: AppHandle,
    state: State<'_, ModelManagerState>,
    selection_mode: RuntimeSelectionMode,
) -> Result<ModelManagerSnapshot, String> {
    aywa_lib::set_runtime_selection_mode(&app, state.inner(), selection_mode)
}

#[tauri::command]
fn select_runtime_profile(
    app: AppHandle,
    state: State<'_, ModelManagerState>,
    profile_id: String,
) -> Result<ModelManagerSnapshot, String> {
    aywa_lib::select_runtime_profile(&app, state.inner(), profile_id)
}

#[tauri::command]
fn apply_runtime_profile_now(
    app: AppHandle,
    state: State<'_, ModelManagerState>,
) -> Result<ModelManagerSnapshot, String> {
    aywa_lib::apply_runtime_profile_now(&app, state.inner())
}

#[tauri::command]
fn test_runtime_profile(
    app: AppHandle,
    state: State<'_, ModelManagerState>,
) -> Result<ModelManagerSnapshot, String> {
    aywa_lib::test_runtime_profile(&app, state.inner())
}

#[tauri::command]
fn redetect_runtime_profile(
    app: AppHandle,
    state: State<'_, ModelManagerState>,
) -> Result<ModelManagerSnapshot, String> {
    aywa_lib::redetect_runtime_profile(&app, state.inner())
}

#[tauri::command]
fn reset_runtime_profile_to_auto(
    app: AppHandle,
    state: State<'_, ModelManagerState>,
) -> Result<ModelManagerSnapshot, String> {
    aywa_lib::reset_runtime_profile_to_auto(&app, state.inner())
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(AppStateManager::default())
        .manage(RuntimeHandles::default())
        .manage(ModelManagerState::default())
        .manage(AppSettingsState::default())
        .setup(|app| {
            match aywa_lib::diagnostics::init_for_app(&app.handle()) {
                Ok(_) => aywa_lib::diagnostics::log("[startup] diagnostics initialized"),
                Err(error) => eprintln!("failed to initialize diagnostics log: {error}"),
            }

            let settings_state: State<'_, AppSettingsState> = app.state();
            let app_settings =
                aywa_lib::startup_app_settings(&app.handle(), settings_state.inner());

            if let Err(error) = aywa_lib::cleanup_recordings_on_startup(&app.handle()) {
                aywa_lib::diagnostics::log(format!("[recordings] startup cleanup failed: {error}"));
            }
            if let Err(error) = aywa_lib::repair_history_on_startup(&app.handle()) {
                aywa_lib::diagnostics::log(format!("[history] startup repair failed: {error}"));
            }

            aywa_lib::configure_windows(&app.handle())?;
            build_tray(&app.handle())?;

            match aywa_lib::register_startup_hotkey(
                &app.handle(),
                settings_state.inner(),
                &app_settings,
            ) {
                Ok(handle) => {
                    let runtime_handles: State<'_, RuntimeHandles> = app.state();
                    *runtime_handles
                        .hotkey
                        .lock()
                        .expect("hotkey mutex poisoned") = Some(handle);
                }
                Err(error) => {
                    aywa_lib::diagnostics::log(format!(
                        "failed to register global hotkey: {error}"
                    ));
                }
            }

            let _ = app.state::<AppStateManager>().emit_current(&app.handle());
            if let Err(error) = aywa_lib::startup_model_manager(&app.handle()) {
                aywa_lib::diagnostics::log(format!(
                    "failed to reconcile model manager state: {error}"
                ));
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            show_settings,
            get_current_state,
            list_history,
            read_history_entry,
            delete_history_entry,
            clear_history,
            get_app_settings,
            set_save_history,
            set_rules,
            set_hotkey,
            show_overlay,
            hide_overlay,
            toggle_overlay,
            list_models,
            open_diagnostics_log,
            preview_issue_report,
            select_model,
            download_model,
            load_model,
            unload_model,
            get_model_status,
            clear_model_cache,
            set_runtime_selection_mode,
            select_runtime_profile,
            apply_runtime_profile_now,
            test_runtime_profile,
            redetect_runtime_profile,
            reset_runtime_profile_to_auto
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
